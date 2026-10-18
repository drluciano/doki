//! Engine scenario and fault-injection tests.
//!
//! Each "device" is a vault plus a save folder in temporary directories,
//! using only synthetic save bytes.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::fs;
use std::panic::{self, AssertUnwindSafe};
use std::path::Path;
use std::rc::Rc;
use std::sync::Once;

use doki_core::{relate, BundleId, CaptureMode, CheckpointId, CheckpointKind, Relation};
use doki_platform::faults::{FaultPlan, FaultyLocation};
use doki_platform::local::LocalDir;
use doki_platform::SaveLocation;
use doki_vault::Vault;
use tempfile::TempDir;

use super::*;

struct FaultProvider {
    plan: Rc<FaultPlan>,
}

impl LocationProvider for FaultProvider {
    fn open<'a>(&'a self, location: &str) -> Box<dyn SaveLocation + 'a> {
        Box::new(FaultyLocation::new(LocalDir::new(location), self.plan.clone()))
    }
}

struct Device {
    _vault_dir: TempDir,
    save_dir: TempDir,
    engine: Engine<FaultProvider>,
    plan: Rc<FaultPlan>,
    bundle: BundleId,
}

type Files = BTreeMap<String, Vec<u8>>;

fn files(entries: &[(&str, &[u8])]) -> Files {
    entries.iter().map(|(p, b)| (p.to_string(), b.to_vec())).collect()
}

fn write_save(dir: &Path, f: &Files) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            fs::remove_dir_all(entry.path()).unwrap();
        } else {
            fs::remove_file(entry.path()).unwrap();
        }
    }
    for (p, bytes) in f {
        let path = dir.join(p);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
}

fn read_save(dir: &Path) -> Files {
    fn walk(root: &Path, dir: &Path, out: &mut Files) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let rel = entry.path().strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
            if rel.starts_with(".doki-staging") {
                continue;
            }
            if entry.file_type().unwrap().is_dir() {
                walk(root, &entry.path(), out);
            } else {
                out.insert(rel, fs::read(entry.path()).unwrap());
            }
        }
    }
    let mut out = Files::new();
    walk(dir, dir, &mut out);
    out
}

fn device(name: &str, bundle: Option<&BundleId>) -> Device {
    let vault_dir = tempfile::tempdir().unwrap();
    let save_dir = tempfile::tempdir().unwrap();
    let plan = FaultPlan::new();
    let vault = Vault::init(vault_dir.path(), name).unwrap();
    let engine = Engine::with_provider(vault, FaultProvider { plan: plan.clone() });
    let record = engine
        .add_bundle("Pokémon Emerald", save_dir.path().to_str().unwrap(), bundle.cloned())
        .unwrap();
    Device { _vault_dir: vault_dir, save_dir, engine, plan, bundle: record.bundle_id }
}

impl Device {
    fn play(&self, f: &Files) {
        write_save(self.save_dir.path(), f);
    }

    fn save(&self) -> Files {
        read_save(self.save_dir.path())
    }

    fn capture(&self) -> CheckpointId {
        self.engine
            .capture(&self.bundle, CaptureMode::EmulatorStoppedConfirmed, None)
            .unwrap()
            .id()
            .clone()
    }

    fn head(&self) -> Option<CheckpointId> {
        self.engine.bundle(&self.bundle).unwrap().head
    }

    fn send_to(&self, other: &Device) -> ImportSummary {
        let dir = tempfile::tempdir().unwrap();
        let pkg = dir.path().join("handoff.dokipkg");
        self.engine.export_package(std::slice::from_ref(&self.bundle), &pkg).unwrap();
        other.engine.import_package(&pkg).unwrap()
    }

    fn checkpoint_files(&self, id: &CheckpointId) -> Files {
        let cp = self.engine.vault().checkpoint(id).unwrap();
        cp.files
            .iter()
            .map(|f| {
                let mut bytes = Vec::new();
                std::io::Read::read_to_end(
                    &mut self.engine.vault().open_object(&f.hash).unwrap(),
                    &mut bytes,
                )
                .unwrap();
                (f.path.to_string(), bytes)
            })
            .collect()
    }

    fn history_ids(&self) -> Vec<CheckpointId> {
        self.engine.history(&self.bundle).unwrap().into_iter().map(|e| e.id).collect()
    }
}

/// PC and handheld sharing one game, PC having captured `pc_files`.
fn pair(pc_files: &Files) -> (Device, Device) {
    let pc = device("PC", None);
    let handheld = device("Handheld", Some(&pc.bundle));
    pc.play(pc_files);
    pc.capture();
    (pc, handheld)
}

// ---- capture -----------------------------------------------------------

#[test]
fn capture_creates_then_deduplicates() {
    let d = device("PC", None);
    d.play(&files(&[("game.sav", b"v1")]));
    let first = d.engine.capture(&d.bundle, CaptureMode::EmulatorStoppedConfirmed, None).unwrap();
    assert!(matches!(first, CaptureOutcome::Created(_)));
    let again = d.engine.capture(&d.bundle, CaptureMode::EmulatorStoppedConfirmed, None).unwrap();
    assert_eq!(again, CaptureOutcome::Unchanged(first.id().clone()));

    d.play(&files(&[("game.sav", b"v2")]));
    let second = d.capture();
    let cp = d.engine.vault().checkpoint(&second).unwrap();
    assert_eq!(cp.parents, vec![first.id().clone()]);
    assert_eq!(d.save(), files(&[("game.sav", b"v2")]), "capture never writes the save");
}

#[test]
fn missing_or_empty_folder_is_never_a_checkpoint() {
    let d = device("PC", None);
    d.play(&files(&[("game.sav", b"v1")]));
    let first = d.capture();

    d.play(&files(&[]));
    let err = d.engine.capture(&d.bundle, CaptureMode::EmulatorStoppedConfirmed, None).unwrap_err();
    assert_eq!(err.code(), "BundleIncomplete");

    let moved = d.save_dir.path().with_extension("moved");
    fs::rename(d.save_dir.path(), &moved).unwrap();
    let err = d.engine.capture(&d.bundle, CaptureMode::EmulatorStoppedConfirmed, None).unwrap_err();
    assert_eq!(err.code(), "SourceUnavailable");
    fs::rename(&moved, d.save_dir.path()).unwrap();

    assert_eq!(d.history_ids(), vec![first.clone()]);
    assert_eq!(d.head(), Some(first));
}

#[test]
fn crash_after_storing_capture_does_not_leave_a_twin() {
    let mut d = device("PC", None);
    d.play(&files(&[("game.sav", b"v1")]));
    d.engine.set_crash_hook(Some(crash_at_name("capture:after-checkpoint")));
    let r = panic::catch_unwind(AssertUnwindSafe(|| d.capture()));
    assert!(r.is_err());
    d.engine.set_crash_hook(None);
    assert_eq!(d.head(), None);
    let id = d.capture();
    assert_eq!(d.history_ids(), vec![id]);
}

// ---- handoff and review --------------------------------------------------

#[test]
fn everyday_handoff_pc_to_handheld() {
    let emerald = files(&[("emerald.sav", b"badge 1")]);
    let (pc, handheld) = pair(&emerald);

    let summary = pc.send_to(&handheld);
    assert_eq!(summary.from_device, "PC");
    assert_eq!(summary.new_proposals.len(), 1);
    assert_eq!(handheld.save(), files(&[]), "receiving never installs");

    let item = &handheld.engine.incoming().unwrap()[0];
    assert_eq!(item.proposal.from_device, "PC");
    let review = handheld.engine.review(&handheld.bundle, &item.proposal.checkpoint).unwrap();
    assert_eq!(review.relation, None, "handheld had no checkpoint of this game");
    let outcome = handheld.engine.use_checkpoint(&review).unwrap();
    assert_eq!(outcome.backup, None);
    assert_eq!(outcome.set_aside, None);
    assert_eq!(handheld.save(), emerald);
    assert_eq!(handheld.head(), Some(pc.head().unwrap()));
    assert!(handheld.engine.incoming().unwrap().is_empty());

    // PC plays on and hands off again: a plain continuation.
    pc.play(&files(&[("emerald.sav", b"badge 2")]));
    pc.capture();
    pc.send_to(&handheld);
    let item = &handheld.engine.incoming().unwrap()[0];
    assert_eq!(item.relation, Some(Relation::Ahead));
    let review = handheld.engine.review(&handheld.bundle, &item.proposal.checkpoint).unwrap();
    let outcome = handheld.engine.use_checkpoint(&review).unwrap();
    assert_eq!(outcome.head, pc.head().unwrap(), "no choice needed for a continuation");
    assert_eq!(outcome.backup.as_ref(), review.baseline_head.as_ref());
    assert_eq!(handheld.save(), files(&[("emerald.sav", b"badge 2")]));
}

#[test]
fn duplicate_delivery_does_not_prompt_twice() {
    let (pc, handheld) = pair(&files(&[("a.sav", b"1")]));
    assert_eq!(pc.send_to(&handheld).new_proposals.len(), 1);
    assert_eq!(pc.send_to(&handheld).new_proposals.len(), 0);
    assert_eq!(handheld.engine.incoming().unwrap().len(), 1);
}

#[test]
fn newer_progress_supersedes_older_proposal() {
    let (pc, handheld) = pair(&files(&[("a.sav", b"1")]));
    pc.send_to(&handheld);
    pc.play(&files(&[("a.sav", b"2")]));
    let newer = pc.capture();
    pc.send_to(&handheld);
    let incoming = handheld.engine.incoming().unwrap();
    assert_eq!(incoming.len(), 1);
    assert_eq!(incoming[0].proposal.checkpoint, newer);
}

#[test]
fn divergence_preserves_both_and_choice_does_not_reappear() {
    let (pc, handheld) = pair(&files(&[("a.sav", b"start")]));
    pc.send_to(&handheld);
    let r = handheld.engine.review(&handheld.bundle, &pc.head().unwrap()).unwrap();
    handheld.engine.use_checkpoint(&r).unwrap();

    // Both play offline.
    pc.play(&files(&[("a.sav", b"pc progress")]));
    let pc_tip = pc.capture();
    handheld.play(&files(&[("a.sav", b"handheld progress")]));
    let handheld_tip = handheld.capture();

    pc.send_to(&handheld);
    let item = &handheld.engine.incoming().unwrap()[0];
    assert!(matches!(item.relation, Some(Relation::Divergent { .. })));
    let review = handheld.engine.review(&handheld.bundle, &pc_tip).unwrap();
    assert!(review.is_divergent());
    let outcome = handheld.engine.use_checkpoint(&review).unwrap();
    assert_eq!(outcome.set_aside, Some(handheld_tip.clone()));
    assert_eq!(outcome.backup, Some(handheld_tip.clone()));
    assert_eq!(handheld.save(), files(&[("a.sav", b"pc progress")]));

    let choice = handheld.engine.vault().checkpoint(&outcome.head).unwrap();
    assert_eq!(choice.kind, CheckpointKind::Choice);
    assert_eq!(choice.parents, vec![pc_tip.clone(), handheld_tip.clone()]);
    let history = handheld.history_ids();
    assert!(history.contains(&handheld_tip) && history.contains(&pc_tip), "both kept");
    assert_eq!(
        handheld.checkpoint_files(&handheld_tip),
        files(&[("a.sav", b"handheld progress")]),
        "set-aside progress is restorable"
    );

    // The PC learns about the choice: same bytes it already has, so no prompt.
    let back = handheld.send_to(&pc);
    assert!(back.new_proposals.is_empty());
    assert_eq!(pc.head(), Some(outcome.head));
    assert!(pc.history_ids().contains(&handheld_tip));
}

#[test]
fn dismiss_keeps_local_and_records_the_choice() {
    let (pc, handheld) = pair(&files(&[("a.sav", b"start")]));
    pc.send_to(&handheld);
    let r = handheld.engine.review(&handheld.bundle, &pc.head().unwrap()).unwrap();
    handheld.engine.use_checkpoint(&r).unwrap();
    pc.play(&files(&[("a.sav", b"pc")]));
    let pc_tip = pc.capture();
    handheld.play(&files(&[("a.sav", b"handheld")]));
    let handheld_tip = handheld.capture();
    pc.send_to(&handheld);

    let choice = handheld.engine.dismiss(&handheld.bundle, &pc_tip).unwrap().unwrap();
    assert_eq!(handheld.save(), files(&[("a.sav", b"handheld")]));
    let cp = handheld.engine.vault().checkpoint(&choice).unwrap();
    assert_eq!(cp.parents, vec![handheld_tip, pc_tip.clone()]);
    assert!(handheld.engine.incoming().unwrap().is_empty());

    // Re-delivery of the dismissed progress does not prompt again.
    assert!(pc.send_to(&handheld).new_proposals.is_empty());
    // The PC is offered the handheld's save, as a continuation of its own.
    handheld.send_to(&pc);
    let item = &pc.engine.incoming().unwrap()[0];
    assert_eq!(item.relation, Some(Relation::Ahead));
}

#[test]
fn keep_alongside_parks_and_reopen_restores() {
    let (pc, handheld) = pair(&files(&[("a.sav", b"1")]));
    pc.send_to(&handheld);
    let cp = pc.head().unwrap();
    handheld.engine.keep_alongside(&handheld.bundle, &cp).unwrap();
    assert!(handheld.engine.incoming().unwrap().is_empty());
    assert!(handheld.history_ids().contains(&cp));
    handheld.engine.reopen(&handheld.bundle, &cp).unwrap();
    assert_eq!(handheld.engine.incoming().unwrap().len(), 1);
}

#[test]
fn review_captures_uncaptured_local_changes_first() {
    let (pc, handheld) = pair(&files(&[("a.sav", b"pc")]));
    handheld.play(&files(&[("a.sav", b"never captured")]));
    pc.send_to(&handheld);
    let review = handheld.engine.review(&handheld.bundle, &pc.head().unwrap()).unwrap();
    let saved = review.captured_local_changes.clone().expect("local changes saved");
    assert_eq!(handheld.checkpoint_files(&saved), files(&[("a.sav", b"never captured")]));
    assert_eq!(review.relation, Some(Relation::Unrelated));
    let outcome = handheld.engine.use_checkpoint(&review).unwrap();
    assert_eq!(outcome.backup, Some(saved));
}

#[test]
fn changes_after_review_void_the_approval() {
    let (pc, handheld) = pair(&files(&[("a.sav", b"pc")]));
    handheld.play(&files(&[("a.sav", b"local")]));
    handheld.capture();
    pc.send_to(&handheld);
    let review = handheld.engine.review(&handheld.bundle, &pc.head().unwrap()).unwrap();

    handheld.play(&files(&[("a.sav", b"played after review")]));
    let err = handheld.engine.use_checkpoint(&review).unwrap_err();
    let Error::LocalChangedSinceReview { new_checkpoint: Some(saved) } = err else {
        panic!("expected LocalChangedSinceReview, got {err:?}");
    };
    assert_eq!(handheld.save(), files(&[("a.sav", b"played after review")]), "untouched");
    assert_eq!(handheld.checkpoint_files(&saved), handheld.save());
    assert_eq!(handheld.head(), Some(saved));
    // A fresh review works.
    let review = handheld.engine.review(&handheld.bundle, &pc.head().unwrap()).unwrap();
    handheld.engine.use_checkpoint(&review).unwrap();
    assert_eq!(handheld.save(), files(&[("a.sav", b"pc")]));
}

#[test]
fn damaged_backup_blocks_replacement() {
    let (pc, handheld) = pair(&files(&[("a.sav", b"pc")]));
    handheld.play(&files(&[("a.sav", b"local")]));
    let local = handheld.capture();
    pc.send_to(&handheld);
    let review = handheld.engine.review(&handheld.bundle, &pc.head().unwrap()).unwrap();
    let cp = handheld.engine.vault().checkpoint(&local).unwrap();
    fs::write(handheld.engine.vault().object_path(&cp.files[0].hash), b"bitrot").unwrap();
    let err = handheld.engine.use_checkpoint(&review).unwrap_err();
    assert_eq!(err.code(), "BackupUnverified");
    assert_eq!(handheld.save(), files(&[("a.sav", b"local")]));
    assert!(handheld.engine.journal(&handheld.bundle).unwrap().is_none());
}

#[test]
fn restore_an_older_checkpoint_keeps_newer_history() {
    let d = device("PC", None);
    d.play(&files(&[("a.sav", b"old")]));
    let old = d.capture();
    d.play(&files(&[("a.sav", b"new")]));
    let new = d.capture();
    let review = d.engine.review(&d.bundle, &old).unwrap();
    assert_eq!(review.relation, Some(Relation::Behind));
    let outcome = d.engine.use_checkpoint(&review).unwrap();
    assert_eq!(d.save(), files(&[("a.sav", b"old")]));
    let choice = d.engine.vault().checkpoint(&outcome.head).unwrap();
    assert_eq!(choice.parents, vec![old, new.clone()]);
    assert!(d.history_ids().contains(&new));
}

#[test]
fn clock_skew_cannot_pick_a_winner() {
    // The handheld's clock is decades behind; its continuation still counts
    // as newer than its parent because ancestry, not time, decides.
    let (pc, handheld) = pair(&files(&[("a.sav", b"1")]));
    pc.send_to(&handheld);
    let base = pc.head().unwrap();
    let base_cp = handheld.engine.vault().checkpoint(&base).unwrap();
    let (hash, size) = handheld.engine.vault().put_object(&mut &b"2"[..], None).unwrap();
    let mut skewed = base_cp.clone();
    skewed.parents = vec![base.clone()];
    skewed.captured_at = "1999-01-01T00:00:00Z".into();
    skewed.files[0].hash = hash;
    skewed.files[0].size = size;
    let skewed_id = handheld.engine.vault().put_checkpoint(&skewed).unwrap();
    assert_eq!(relate(handheld.engine.vault(), &base, &skewed_id), Relation::Ahead);
}

#[test]
fn unmapped_game_waits_for_explicit_setup() {
    let pc = device("PC", None);
    pc.play(&files(&[("a.sav", b"1")]));
    pc.capture();
    let other = device("Handheld", None); // a different game
    let summary = pc.send_to(&other);
    assert!(summary.new_proposals.is_empty());
    assert_eq!(summary.unmapped, vec![(pc.bundle.clone(), "Pokémon Emerald".to_string())]);
    assert_eq!(other.engine.unmapped_bundles().unwrap().len(), 1);

    let folder = tempfile::tempdir().unwrap();
    other
        .engine
        .add_bundle("Emerald", folder.path().to_str().unwrap(), Some(pc.bundle.clone()))
        .unwrap();
    assert!(other.engine.unmapped_bundles().unwrap().is_empty());
    assert_eq!(other.engine.incoming().unwrap().len(), 1);
}

#[test]
fn tampered_or_truncated_package_is_rejected_without_partial_history() {
    let (pc, handheld) = pair(&files(&[("a.sav", b"some save bytes")]));
    let dir = tempfile::tempdir().unwrap();
    let pkg = dir.path().join("p.dokipkg");
    pc.engine.export_package(std::slice::from_ref(&pc.bundle), &pkg).unwrap();
    let bytes = fs::read(&pkg).unwrap();

    let mut flipped = bytes.clone();
    *flipped.last_mut().unwrap() ^= 0xff;
    let bad = dir.path().join("flipped.dokipkg");
    fs::write(&bad, &flipped).unwrap();
    assert_eq!(handheld.engine.import_package(&bad).unwrap_err().code(), "IntegrityFailed");

    let short = dir.path().join("short.dokipkg");
    fs::write(&short, &bytes[..bytes.len() - 3]).unwrap();
    assert_eq!(handheld.engine.import_package(&short).unwrap_err().code(), "IntegrityFailed");

    let junk = dir.path().join("junk.dokipkg");
    fs::write(&junk, b"hello").unwrap();
    assert_eq!(handheld.engine.import_package(&junk).unwrap_err().code(), "InvalidInput");

    assert!(handheld.engine.vault().checkpoints().unwrap().is_empty());
    assert!(handheld.engine.incoming().unwrap().is_empty());
    handheld.engine.import_package(&pkg).unwrap();
}

#[test]
fn folders_cannot_overlap() {
    let d = device("PC", None);
    let err = d.engine.add_bundle("Again", d.save_dir.path().to_str().unwrap(), None).unwrap_err();
    assert_eq!(err.code(), "InvalidInput");
    let inside_vault = d.engine.vault().root().join("records");
    let err = d.engine.add_bundle("Bad", inside_vault.to_str().unwrap(), None).unwrap_err();
    assert_eq!(err.code(), "InvalidInput");
}

// ---- fault injection -----------------------------------------------------

#[derive(Debug)]
struct SimulatedCrash;

fn quiet_crashes() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let default = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            if info.payload().downcast_ref::<SimulatedCrash>().is_none() {
                default(info);
            }
        }));
    });
}

fn crash_at_name(name: &'static str) -> CrashHook {
    quiet_crashes();
    Box::new(move |point| {
        if point == name {
            panic::panic_any(SimulatedCrash);
        }
    })
}

/// Crash on the `n`th crash point reached (0-based); count points seen.
fn crash_at_index(n: usize, seen: Rc<Cell<usize>>) -> CrashHook {
    quiet_crashes();
    Box::new(move |_| {
        let i = seen.get();
        seen.set(i + 1);
        if i == n {
            panic::panic_any(SimulatedCrash);
        }
    })
}

/// A divergent, multi-file install: one file changes, one stays, one is
/// removed and one is added. Returns (handheld, review, local files,
/// incoming files, local tip, incoming tip).
fn divergent_setup() -> (Device, Review, Files, Files, CheckpointId, CheckpointId) {
    let pc = device("PC", None);
    let handheld = device("Handheld", Some(&pc.bundle));
    let base = files(&[("main.sav", b"base"), ("keep.cfg", b"same")]);
    pc.play(&base);
    pc.capture();
    pc.send_to(&handheld);
    let r = handheld.engine.review(&handheld.bundle, &pc.head().unwrap()).unwrap();
    handheld.engine.use_checkpoint(&r).unwrap();

    let local =
        files(&[("main.sav", b"handheld"), ("keep.cfg", b"same"), ("slot/old.dat", b"old")]);
    handheld.play(&local);
    let local_tip = handheld.capture();
    let incoming = files(&[("main.sav", b"pc"), ("keep.cfg", b"same"), ("slot/new.dat", b"new")]);
    pc.play(&incoming);
    let incoming_tip = pc.capture();
    pc.send_to(&handheld);
    let review = handheld.engine.review(&handheld.bundle, &incoming_tip).unwrap();
    (handheld, review, local, incoming, local_tip, incoming_tip)
}

/// After any interruption the save must be exactly one of the two complete
/// states, or recoverable into either, and both must remain in history.
fn check_recoverable(
    d: &Device,
    local: &Files,
    incoming: &Files,
    local_tip: &CheckpointId,
    incoming_tip: &CheckpointId,
    action: RecoveryAction,
) {
    let history = d.history_ids();
    assert!(history.contains(local_tip) && history.contains(incoming_tip));
    match d.engine.journal(&d.bundle).unwrap() {
        None => {
            let now = d.save();
            assert!(now == *local || now == *incoming, "no journal, so the save must be whole");
            if now == *incoming {
                let head = d.head().unwrap();
                assert_eq!(d.checkpoint_files(&head), *incoming, "head matches files");
            } else {
                assert_eq!(d.head().as_ref(), Some(local_tip));
            }
        }
        Some(journal) => {
            let status = d.engine.recovery_status(&d.bundle).unwrap().unwrap();
            assert_eq!(status.journal, journal);
            let outcome = d.engine.recover(&d.bundle, action).unwrap();
            assert!(d.engine.journal(&d.bundle).unwrap().is_none());
            match &outcome {
                RecoveryOutcome::NothingChanged => assert_eq!(d.save(), *local),
                RecoveryOutcome::Finished { head } => {
                    assert_eq!(d.save(), *incoming);
                    assert_eq!(d.checkpoint_files(head), *incoming);
                }
                RecoveryOutcome::PutBack { head } => {
                    assert_eq!(d.save(), *local);
                    assert_eq!(head.as_ref(), Some(local_tip));
                    assert_eq!(d.head().as_ref(), Some(local_tip));
                }
            }
            // A folder already equal to the journal's target is committed as
            // is, whatever was asked; the target may already be the backup
            // if an earlier recovery chose to put it back.
            if status.complete && matches!(journal.purpose, records::InstallPurpose::Use { .. }) {
                assert!(matches!(outcome, RecoveryOutcome::Finished { .. }));
            }
        }
    }
}

#[test]
fn crash_at_every_point_of_an_install_is_recoverable_both_ways() {
    // Count the crash points one clean run passes through.
    let (mut d, review, ..) = divergent_setup();
    let seen = Rc::new(Cell::new(0));
    d.engine.set_crash_hook(Some(crash_at_index(usize::MAX, seen.clone())));
    d.engine.use_checkpoint(&review).unwrap();
    let points = seen.get();
    assert!(points >= 8, "expected many crash points, saw {points}");

    for action in [RecoveryAction::Finish, RecoveryAction::PutBack] {
        for n in 0..points {
            let (mut d, review, local, incoming, local_tip, incoming_tip) = divergent_setup();
            d.engine.set_crash_hook(Some(crash_at_index(n, Rc::new(Cell::new(0)))));
            let result = panic::catch_unwind(AssertUnwindSafe(|| d.engine.use_checkpoint(&review)));
            assert!(result.is_err(), "crash point {n} should have fired");
            d.engine.set_crash_hook(None);
            check_recoverable(&d, &local, &incoming, &local_tip, &incoming_tip, action);
        }
    }
}

#[test]
fn io_failure_at_every_save_folder_write_is_safe() {
    let (d, review, ..) = divergent_setup();
    let before = d.plan.calls();
    d.engine.use_checkpoint(&review).unwrap();
    let writes = d.plan.calls() - before;
    assert!(writes >= 5, "stage x2, commit x2, remove, discard; saw {writes}");

    for action in [RecoveryAction::Finish, RecoveryAction::PutBack] {
        for n in 0..writes {
            let (d, review, local, incoming, local_tip, incoming_tip) = divergent_setup();
            d.plan.fail_from(d.plan.calls() + n);
            let result = d.engine.use_checkpoint(&review);
            d.plan.disarm();
            match result {
                Ok(_) => assert_eq!(d.save(), incoming, "only cleanup failed"),
                Err(e) if e.playable_files_may_be_mixed() => {
                    assert!(d.engine.journal(&d.bundle).unwrap().is_some())
                }
                Err(_) => {
                    assert_eq!(d.save(), local, "failed before any write: untouched");
                    assert!(d.engine.journal(&d.bundle).unwrap().is_none());
                }
            }
            check_recoverable(&d, &local, &incoming, &local_tip, &incoming_tip, action);
        }
    }
}

#[test]
fn crash_during_recovery_is_itself_recoverable() {
    for action in [RecoveryAction::Finish, RecoveryAction::PutBack] {
        // Interrupt the install right before the first playable write
        // completes, then interrupt recovery at each of its points.
        let mut n = 0;
        loop {
            let (mut d, review, local, incoming, local_tip, incoming_tip) = divergent_setup();
            d.engine.set_crash_hook(Some(crash_at_name("install:after-file")));
            let _ = panic::catch_unwind(AssertUnwindSafe(|| d.engine.use_checkpoint(&review)));
            let seen = Rc::new(Cell::new(0));
            d.engine.set_crash_hook(Some(crash_at_index(n, seen.clone())));
            let r = panic::catch_unwind(AssertUnwindSafe(|| d.engine.recover(&d.bundle, action)));
            d.engine.set_crash_hook(None);
            let crashed = r.is_err();
            check_recoverable(&d, &local, &incoming, &local_tip, &incoming_tip, action);
            if !crashed {
                break;
            }
            n += 1;
        }
        assert!(n > 0, "recovery passed through crash points");
    }
}

#[test]
fn files_written_during_an_interrupted_install_are_rescued() {
    let (mut d, review, local, _incoming, local_tip, _) = divergent_setup();
    d.engine.set_crash_hook(Some(crash_at_name("install:after-file")));
    let _ = panic::catch_unwind(AssertUnwindSafe(|| d.engine.use_checkpoint(&review)));
    d.engine.set_crash_hook(None);

    // The user opened the emulator before Doki recovered.
    fs::write(d.save_dir.path().join("main.sav"), b"emulator wrote this").unwrap();
    let status = d.engine.recovery_status(&d.bundle).unwrap().unwrap();
    assert!(status.files.iter().any(|(_, s)| *s == FileState::Other));
    assert_eq!(
        d.engine
            .capture(&d.bundle, CaptureMode::EmulatorStoppedConfirmed, None)
            .unwrap_err()
            .code(),
        "RecoveryRequired"
    );

    let outcome = d.engine.recover(&d.bundle, RecoveryAction::PutBack).unwrap();
    assert_eq!(outcome, RecoveryOutcome::PutBack { head: Some(local_tip) });
    assert_eq!(d.save(), local);
    let rescued = d
        .engine
        .history(&d.bundle)
        .unwrap()
        .into_iter()
        .find(|e| e.checkpoint.kind == CheckpointKind::Backup)
        .expect("mixed state kept in history");
    assert_eq!(d.checkpoint_files(&rescued.id)["main.sav"], b"emulator wrote this");
}

#[test]
fn clean_leftovers_never_removes_checkpoint_content() {
    let (d, review, _, incoming, ..) = divergent_setup();
    d.engine.use_checkpoint(&review).unwrap();
    let (orphan, _) = d.engine.vault().put_object(&mut &b"orphan"[..], None).unwrap();
    assert!(d.engine.clean_leftovers().unwrap() >= 6);
    assert!(!d.engine.vault().has_object(&orphan));
    for e in d.engine.history(&d.bundle).unwrap() {
        d.engine.vault().verify_checkpoint_objects(&e.checkpoint).unwrap();
    }
    assert_eq!(d.save(), incoming);
}
