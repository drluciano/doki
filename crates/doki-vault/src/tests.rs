use super::*;
use doki_core::{
    BundleId, BundlePath, CaptureMode, CheckpointKind, ProfileRef, CHECKPOINT_SCHEMA_VERSION,
};

fn checkpoint(vault: &Vault, content: &[u8], parents: Vec<CheckpointId>) -> Checkpoint {
    let (hash, size) = vault.put_object(&mut &content[..], None).unwrap();
    Checkpoint {
        schema_version: CHECKPOINT_SCHEMA_VERSION,
        bundle_id: BundleId::parse("0011223344556677").unwrap(),
        parents,
        kind: CheckpointKind::Capture,
        files: vec![FileEntry { path: BundlePath::new("game.sav").unwrap(), size, hash }],
        profile: ProfileRef { id: "generic-folder".into(), version: 1 },
        origin_device: vault.device_id().clone(),
        origin_device_name: vault.device_name().to_owned(),
        captured_at: now_rfc3339(),
        capture_mode: CaptureMode::EmulatorStoppedConfirmed,
        note: None,
    }
}

#[test]
fn init_then_open() {
    let tmp = tempfile::tempdir().unwrap();
    let v = Vault::init(tmp.path(), "PC").unwrap();
    let id = v.device_id().clone();
    assert!(Vault::init(tmp.path(), "PC").is_err(), "never re-initialize over a vault");
    let v2 = Vault::open(tmp.path()).unwrap();
    assert_eq!(v2.device_id(), &id);
    assert_eq!(v2.device_name(), "PC");
}

#[test]
fn objects_are_deduplicated_and_verified() {
    let tmp = tempfile::tempdir().unwrap();
    let v = Vault::init(tmp.path(), "PC").unwrap();
    let (h1, n) = v.put_object(&mut &b"hello"[..], None).unwrap();
    let (h2, _) = v.put_object(&mut &b"hello"[..], None).unwrap();
    assert_eq!(h1, h2);
    assert_eq!(n, 5);

    let wrong = ContentHash::of_bytes(b"other");
    let err = v.put_object(&mut &b"hello"[..], Some((&wrong, 5))).unwrap_err();
    assert_eq!(err.code(), "IntegrityFailed");
    let err = v.put_object(&mut &b"hello!"[..], Some((&h1, 5))).unwrap_err();
    assert_eq!(err.code(), "IntegrityFailed");
    assert_eq!(fs::read_dir(tmp.path().join("tmp")).unwrap().count(), 0, "no temp leftovers");
}

#[test]
fn damaged_object_is_detected() {
    let tmp = tempfile::tempdir().unwrap();
    let v = Vault::init(tmp.path(), "PC").unwrap();
    let cp = checkpoint(&v, b"progress", vec![]);
    fs::write(v.object_path(&cp.files[0].hash), b"bitrot!!").unwrap();
    assert_eq!(v.verify_checkpoint_objects(&cp).unwrap_err().code(), "IntegrityFailed");
}

#[test]
fn checkpoint_requires_objects_and_detects_tampering() {
    let tmp = tempfile::tempdir().unwrap();
    let v = Vault::init(tmp.path(), "PC").unwrap();
    let cp = checkpoint(&v, b"progress", vec![]);
    let id = v.put_checkpoint(&cp).unwrap();
    assert_eq!(v.put_checkpoint(&cp).unwrap(), id, "idempotent");
    assert_eq!(v.checkpoint(&id).unwrap(), cp);

    let mut dangling = cp.clone();
    dangling.files[0].hash = ContentHash::of_bytes(b"never stored");
    assert_eq!(v.put_checkpoint(&dangling).unwrap_err().code(), "IntegrityFailed");

    let path = tmp.path().join("checkpoints").join(format!("{id}.json"));
    let tampered = fs::read_to_string(&path).unwrap().replace("generic-folder", "evil-folder");
    fs::write(&path, tampered).unwrap();
    assert_eq!(v.checkpoint(&id).unwrap_err().code(), "IntegrityFailed");
}

#[test]
fn graph_reads_parents() {
    let tmp = tempfile::tempdir().unwrap();
    let v = Vault::init(tmp.path(), "PC").unwrap();
    let a = v.put_checkpoint(&checkpoint(&v, b"a", vec![])).unwrap();
    let b = v.put_checkpoint(&checkpoint(&v, b"b", vec![a.clone()])).unwrap();
    assert_eq!(v.parents(&b), Some(vec![a.clone()]));
    assert_eq!(doki_core::relate(&v, &a, &b), doki_core::Relation::Ahead);
    let unknown = CheckpointId::parse("ffffffffffffffff").unwrap();
    assert_eq!(v.parents(&unknown), None);
}

#[test]
fn records_round_trip_and_reject_unsafe_keys() {
    let tmp = tempfile::tempdir().unwrap();
    let v = Vault::init(tmp.path(), "PC").unwrap();
    v.put_record("bundles", "abc", &vec![1, 2, 3]).unwrap();
    assert_eq!(v.get_record::<Vec<i32>>("bundles", "abc").unwrap(), Some(vec![1, 2, 3]));
    assert_eq!(v.list_records::<Vec<i32>>("bundles").unwrap().len(), 1);
    v.delete_record("bundles", "abc").unwrap();
    assert_eq!(v.get_record::<Vec<i32>>("bundles", "abc").unwrap(), None);
    assert!(v.put_record("bundles", "../escape", &1).is_err());
    assert!(v.list_records::<i32>("nothing-yet").unwrap().is_empty());
}

#[test]
fn lock_is_exclusive() {
    let tmp = tempfile::tempdir().unwrap();
    let v = Vault::init(tmp.path(), "PC").unwrap();
    let held = v.lock("bundle-1").unwrap();
    assert_eq!(v.lock("bundle-1").err().map(|e| e.code()), Some("Busy"));
    drop(held);
    v.lock("bundle-1").unwrap();
}

#[test]
fn sweep_keeps_referenced_and_protected_objects() {
    let tmp = tempfile::tempdir().unwrap();
    let v = Vault::init(tmp.path(), "PC").unwrap();
    let cp = checkpoint(&v, b"kept", vec![]);
    v.put_checkpoint(&cp).unwrap();
    let (orphan, _) = v.put_object(&mut &b"orphan"[..], None).unwrap();
    let (protected, _) = v.put_object(&mut &b"protected"[..], None).unwrap();
    fs::write(tmp.path().join("tmp").join("leftover"), b"x").unwrap();

    let freed = v.sweep_unreferenced(&HashSet::from([protected.clone()])).unwrap();
    assert_eq!(freed, 6 + 1);
    assert!(v.has_object(&cp.files[0].hash));
    assert!(v.has_object(&protected));
    assert!(!v.has_object(&orphan));
}
