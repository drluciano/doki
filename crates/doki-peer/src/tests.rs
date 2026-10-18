use std::path::{Path, PathBuf};

use doki_core::{BundleId, CaptureMode};
use doki_engine::Engine;
use doki_vault::Vault;

use super::*;

struct Device {
    vault: PathBuf,
    save: PathBuf,
    node: PeerNode,
}

impl Device {
    fn new(root: &Path, name: &str) -> Self {
        let vault = root.join(format!("{name}-vault"));
        let save = root.join(format!("{name}-save"));
        std::fs::create_dir_all(&save).unwrap();
        Vault::init(&vault, name).unwrap();
        let node = PeerNode::start(&vault, Network::Loopback).unwrap();
        Device { vault, save, node }
    }

    fn engine(&self) -> Engine {
        Engine::open(&self.vault).unwrap()
    }

    fn add_game(&self, name: &str, link: Option<&BundleId>) -> BundleId {
        self.engine()
            .add_bundle(name, self.save.to_str().unwrap(), link.cloned())
            .unwrap()
            .bundle_id
    }

    fn play(&self, game: &BundleId, bytes: &[u8]) {
        std::fs::write(self.save.join("game.sav"), bytes).unwrap();
        self.engine().capture(game, CaptureMode::EmulatorStoppedConfirmed, None).unwrap();
    }

    fn incoming(&self) -> usize {
        self.engine().incoming().unwrap().len()
    }

    fn peer_names(&self) -> Vec<String> {
        self.node.peers().unwrap().into_iter().map(|p| p.name).collect()
    }
}

#[test]
fn pairing_spreads_trust_and_progress_moves_directly() {
    let tmp = tempfile::tempdir().unwrap();
    let (pc, odin, deck) = (
        Device::new(tmp.path(), "PC"),
        Device::new(tmp.path(), "Odin"),
        Device::new(tmp.path(), "Deck"),
    );
    let game = pc.add_game("Emerald", None);
    pc.play(&game, b"one");

    // Odin joins PC and receives the game right away, as unmapped progress.
    assert_eq!(odin.node.join(&pc.node.invite().unwrap().code).unwrap(), "PC");
    assert_eq!(odin.peer_names(), ["PC"]);
    assert_eq!(pc.peer_names(), ["Odin"]);
    let unmapped = odin.engine().unmapped_bundles().unwrap();
    assert_eq!(unmapped.len(), 1);
    assert_eq!(unmapped[0].bundle_id, game);

    // Receiving never touches the save folder.
    odin.add_game("Emerald", Some(&game));
    assert!(!odin.save.join("game.sav").exists());
    assert_eq!(odin.incoming(), 1);

    // Deck joins through Odin and learns about PC from Odin's book.
    deck.node.join(&odin.node.invite().unwrap().code).unwrap();
    assert_eq!(deck.peer_names(), ["Odin", "PC"]);
    // PC learns about Deck the next time it syncs with Odin.
    pc.node.sync_all().unwrap();
    assert_eq!(pc.peer_names(), ["Deck", "Odin"]);

    // Now Deck and PC sync directly.
    let report = deck.node.sync_all().unwrap();
    assert!(report.unreachable.is_empty(), "{:?}", report.unreachable);
    assert_eq!(report.sessions.len(), 2);
}

#[test]
fn both_devices_played_and_each_sees_the_other() {
    let tmp = tempfile::tempdir().unwrap();
    let (pc, odin) = (Device::new(tmp.path(), "PC"), Device::new(tmp.path(), "Odin"));
    let game = pc.add_game("Emerald", None);
    pc.play(&game, b"one");
    odin.node.join(&pc.node.invite().unwrap().code).unwrap();
    odin.add_game("Emerald", Some(&game));

    // Odin uses PC's save, then both play separately.
    let engine = odin.engine();
    let item = &engine.incoming().unwrap()[0];
    let review = engine.review(&game, &item.proposal.checkpoint).unwrap();
    engine.use_checkpoint(&review).unwrap();
    odin.play(&game, b"odin progress");
    pc.play(&game, b"pc progress");

    let report = pc.node.sync_all().unwrap();
    assert_eq!(report.sessions[0].sent, 1);
    assert_eq!(pc.incoming(), 1);
    assert_eq!(odin.incoming(), 1);
    assert_eq!(std::fs::read(pc.save.join("game.sav")).unwrap(), b"pc progress");
    assert_eq!(std::fs::read(odin.save.join("game.sav")).unwrap(), b"odin progress");

    // Nothing changed: a second sync sends nothing.
    let again = odin.node.sync_all().unwrap();
    assert_eq!(again.sessions[0].sent, 0);
    assert!(again.sessions[0].received.is_none());
}

#[test]
fn removed_device_is_shut_out_everywhere() {
    let tmp = tempfile::tempdir().unwrap();
    let (pc, odin, lost) = (
        Device::new(tmp.path(), "PC"),
        Device::new(tmp.path(), "Odin"),
        Device::new(tmp.path(), "Lost"),
    );
    odin.node.join(&pc.node.invite().unwrap().code).unwrap();
    lost.node.join(&pc.node.invite().unwrap().code).unwrap();
    odin.node.sync_all().unwrap();
    assert_eq!(odin.peer_names(), ["Lost", "PC"]);

    let lost_id = lost.node.id().to_owned();
    pc.node.remove(&lost_id).unwrap();
    assert_eq!(pc.peer_names(), ["Odin"]);
    // The removal reaches Odin on its next sync with PC.
    odin.node.sync_all().unwrap();
    assert_eq!(odin.peer_names(), ["PC"]);

    // The removed device can reach neither.
    let report = lost.node.sync_all().unwrap();
    assert!(report.sessions.is_empty());
    assert_eq!(report.unreachable.len(), 2);
    // And PC never re-adds it from someone else's book.
    pc.node.sync_all().unwrap();
    assert_eq!(pc.peer_names(), ["Odin"]);
}

#[test]
fn bad_or_expired_codes_are_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let (pc, odin) = (Device::new(tmp.path(), "PC"), Device::new(tmp.path(), "Odin"));
    assert!(matches!(odin.node.join("hello"), Err(Error::InvalidInput(_))));
    let code = pc.node.invite().unwrap().code;
    assert!(matches!(pc.node.join(&code), Err(Error::InvalidInput(_))));
    pc.node.ctx.invites.lock().unwrap().clear();
    assert!(matches!(odin.node.join(&code), Err(Error::PermissionLost(_))));
    assert!(odin.peer_names().is_empty());
    assert!(pc.peer_names().is_empty());
}

#[test]
fn identity_survives_restart() {
    let tmp = tempfile::tempdir().unwrap();
    let pc = Device::new(tmp.path(), "PC");
    let id = pc.node.id().to_owned();
    let vault = pc.vault.clone();
    pc.node.shutdown();
    let again = PeerNode::start(&vault, Network::Loopback).unwrap();
    assert_eq!(again.id(), id);
}

#[test]
fn short_code_pairs_devices_on_the_same_network() {
    let tmp = tempfile::tempdir().unwrap();
    let port = 48_000 + (std::process::id() % 900) as u16;
    let start = |name: &str| {
        let vault = tmp.path().join(name);
        Vault::init(&vault, name).unwrap();
        PeerNode::start_on(&vault, Network::Loopback, port).unwrap()
    };
    let (pc, odin) = (start("PC"), start("Odin"));
    // No one is showing a code yet.
    let nobody = odin.join("2345-6789");
    assert!(matches!(nobody, Err(Error::SourceUnavailable(_))), "{nobody:?}");
    let invitation = pc.invite().unwrap();
    assert_eq!(invitation.short.len(), 9);
    // A wrong short code is refused, a lowercase right one works.
    let wrong = odin.join("2345-6789");
    assert!(matches!(wrong, Err(Error::PermissionLost(_))), "{wrong:?}");
    assert_eq!(odin.join(&invitation.short.to_lowercase()).unwrap(), "PC");
    assert_eq!(pc.peers().unwrap()[0].name, "Odin");
}

#[test]
fn a_removed_device_can_be_paired_again() {
    let tmp = tempfile::tempdir().unwrap();
    let (pc, odin, lost) = (
        Device::new(tmp.path(), "PC"),
        Device::new(tmp.path(), "Odin"),
        Device::new(tmp.path(), "Found"),
    );
    odin.node.join(&pc.node.invite().unwrap().code).unwrap();
    lost.node.join(&pc.node.invite().unwrap().code).unwrap();
    odin.node.sync_all().unwrap();
    pc.node.remove(lost.node.id()).unwrap();
    odin.node.sync_all().unwrap();
    assert_eq!(odin.peer_names(), ["PC"]);

    // Found again: pair it through Odin. Times have one-second resolution.
    std::thread::sleep(std::time::Duration::from_millis(1100));
    lost.node.join(&odin.node.invite().unwrap().code).unwrap();
    // PC learns of the newer pairing from Odin instead of keeping the removal.
    pc.node.sync_all().unwrap();
    assert_eq!(pc.peer_names(), ["Found", "Odin"]);
    let report = lost.node.sync_all().unwrap();
    assert!(report.unreachable.is_empty(), "{:?}", report.unreachable);
}
