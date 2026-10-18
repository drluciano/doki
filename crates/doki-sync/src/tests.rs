use std::fs;
use std::path::Path;

use doki_core::CaptureMode;
use doki_engine::Engine;
use doki_relay::{Relay, RelayConfig};
use doki_vault::Vault;
use tempfile::TempDir;

use super::*;

fn start_relay() -> (TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let relay = Relay::bind(RelayConfig::new(dir.path()), "127.0.0.1:0").unwrap();
    let url = format!("http://{}", relay.local_addr());
    std::thread::spawn(move || relay.run());
    (dir, url)
}

struct Device {
    _dirs: (TempDir, TempDir),
    save: std::path::PathBuf,
    engine: Engine,
}

fn device(name: &str) -> Device {
    let vault_dir = tempfile::tempdir().unwrap();
    let save_dir = tempfile::tempdir().unwrap();
    Vault::init(vault_dir.path(), name).unwrap();
    let engine = Engine::open(vault_dir.path()).unwrap();
    let save = save_dir.path().to_path_buf();
    Device { _dirs: (vault_dir, save_dir), save, engine }
}

fn write(dir: &Path, bytes: &[u8]) {
    fs::write(dir.join("game.sav"), bytes).unwrap();
}

#[test]
fn pairing_code_round_trips_and_rejects_junk() {
    let link = RelayLink::create("https://relay.example/").unwrap();
    let back = RelayLink::from_pairing_code(&link.pairing_code()).unwrap();
    assert_eq!(back.url, "https://relay.example");
    assert_eq!(back.space, link.space);
    assert_eq!(back.token(), link.token());
    for bad in ["", "DOKI1:", "DOKI1:https://x#abc.def", "hello", "DOKI1:ftp://x#00.00"] {
        assert!(RelayLink::from_pairing_code(bad).is_err(), "{bad:?}");
    }
    assert!(!format!("{link:?}").contains(&link.secret), "secret never printed");
}

#[test]
fn sealed_packages_need_the_right_key_and_detect_tampering() {
    let a = RelayLink::create("http://r").unwrap();
    let b = RelayLink::create("http://r").unwrap();
    let blob = a.seal(b"save bytes");
    assert_eq!(a.open(&blob).unwrap(), b"save bytes");
    assert!(b.open(&blob).is_err(), "other space cannot read");
    let mut flipped = blob.clone();
    *flipped.last_mut().unwrap() ^= 1;
    assert_eq!(a.open(&flipped).unwrap_err().code(), "IntegrityFailed");
    assert!(!blob.windows(10).any(|w| w == b"save bytes"));
    assert_ne!(
        a.token(),
        hex::encode(a.derive(b"doki package content key v1")),
        "keys are separate"
    );
}

#[test]
fn two_devices_hand_off_through_the_relay_without_installing() {
    let (relay_dir, url) = start_relay();
    let pc = device("PC");
    let odin = device("Odin");

    let link = RelayLink::create(&url).unwrap();
    connect(&link).unwrap();
    save_link(&pc.engine, &link).unwrap();
    let joined = RelayLink::from_pairing_code(&link.pairing_code()).unwrap();
    connect(&joined).unwrap();
    save_link(&odin.engine, &joined).unwrap();

    let game = pc.engine.add_bundle("Emerald", pc.save.to_str().unwrap(), None).unwrap();
    write(&pc.save, b"secret progress bytes");
    pc.engine.capture(&game.bundle_id, CaptureMode::EmulatorStoppedConfirmed, None).unwrap();

    let report = sync(&pc.engine).unwrap();
    assert_eq!(report.uploaded, vec![game.bundle_id.clone()]);
    assert!(sync(&pc.engine).unwrap().uploaded.is_empty(), "unchanged saves are not re-sent");

    // The relay holds only ciphertext.
    for entry in walk(relay_dir.path()) {
        let bytes = fs::read(&entry).unwrap();
        assert!(
            !bytes.windows(21).any(|w| w == b"secret progress bytes"),
            "{entry:?} leaks content"
        );
        assert!(!bytes.windows(7).any(|w| w == b"Emerald"), "{entry:?} leaks game name");
    }

    odin.engine
        .add_bundle("Emerald", odin.save.to_str().unwrap(), Some(game.bundle_id.clone()))
        .unwrap();
    let report = sync(&odin.engine).unwrap();
    assert_eq!(report.received.len(), 1);
    assert_eq!(report.received[0].from_device, "PC");
    assert!(fs::read_dir(&odin.save).unwrap().next().is_none(), "receiving never installs");
    let incoming = odin.engine.incoming().unwrap();
    assert_eq!(incoming.len(), 1);

    let review = odin.engine.review(&game.bundle_id, &incoming[0].proposal.checkpoint).unwrap();
    odin.engine.use_checkpoint(&review).unwrap();
    assert_eq!(fs::read(odin.save.join("game.sav")).unwrap(), b"secret progress bytes");

    // A second sync fetches nothing new.
    assert!(sync(&odin.engine).unwrap().received.is_empty());
}

#[test]
fn wrong_pairing_is_refused() {
    let (_dir, url) = start_relay();
    let link = RelayLink::create(&url).unwrap();
    connect(&link).unwrap();
    let mut impostor = link.clone();
    impostor.secret = "11".repeat(32);
    assert_eq!(connect(&impostor).unwrap_err().code(), "PermissionLost");
}

#[test]
fn unreachable_relay_is_reported_plainly() {
    let link = RelayLink::create("http://127.0.0.1:9").unwrap();
    assert_eq!(connect(&link).unwrap_err().code(), "SourceUnavailable");
}

fn walk(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for e in fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            out.extend(walk(&p));
        } else {
            out.push(p);
        }
    }
    out
}
