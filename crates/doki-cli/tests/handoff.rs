//! End-to-end: the MVP milestone from the brief, driven through the `doki`
//! binary with synthetic saves. Play on both devices, detect divergence,
//! preserve both saves, choose one, and restore the other.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn doki(vault: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_doki"))
        .arg("--vault")
        .arg(vault)
        .args(args)
        .output()
        .expect("run doki")
}

fn ok(vault: &Path, args: &[&str]) -> String {
    let out = doki(vault, args);
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        out.status.success(),
        "doki {args:?} failed:\n{text}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    text
}

/// First 8-hex-character token after `marker` in `text`.
fn short_id_after(text: &str, marker: &str) -> String {
    let rest =
        &text[text.find(marker).unwrap_or_else(|| panic!("{marker:?} in {text}")) + marker.len()..];
    rest.split(|c: char| !c.is_ascii_hexdigit()).find(|t| t.len() == 8).unwrap().to_owned()
}

#[test]
fn play_on_both_devices_choose_one_restore_the_other() {
    let tmp = tempfile::tempdir().unwrap();
    let (pc_vault, odin_vault) = (tmp.path().join("pc-vault"), tmp.path().join("odin-vault"));
    let (pc_save, odin_save) = (tmp.path().join("pc-save"), tmp.path().join("odin-save"));
    fs::create_dir_all(&pc_save).unwrap();
    fs::create_dir_all(&odin_save).unwrap();
    let pkg = |n: &str| tmp.path().join(n).to_string_lossy().to_string();

    ok(&pc_vault, &["init", "--name", "PC"]);
    ok(&odin_vault, &["init", "--name", "Odin"]);
    let added = ok(&pc_vault, &["add", "Emerald", pc_save.to_str().unwrap()]);
    assert!(added.contains("Set up Emerald"));

    // Play on Windows, capture, hand off.
    fs::write(pc_save.join("emerald.sav"), b"gym 1").unwrap();
    ok(&pc_vault, &["capture", "Emerald", "--closed"]);
    ok(&pc_vault, &["send", "Emerald", "-o", &pkg("1.dokipkg")]);

    // Handheld receives; the game is not set up there yet.
    let received = ok(&odin_vault, &["receive", &pkg("1.dokipkg")]);
    assert!(received.contains("Your saves were not changed"));
    let bundle = received
        .split("--link ")
        .nth(1)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .trim_end_matches('`')
        .to_owned();
    ok(&odin_vault, &["add", "Emerald", odin_save.to_str().unwrap(), "--link", &bundle]);
    let incoming = ok(&odin_vault, &["incoming"]);
    assert!(incoming.contains("from PC"));
    let first = incoming.split_whitespace().next().unwrap().to_owned();
    ok(&odin_vault, &["use", "Emerald", &first, "--yes"]);
    assert_eq!(fs::read(odin_save.join("emerald.sav")).unwrap(), b"gym 1");

    // Both devices play independently.
    fs::write(pc_save.join("emerald.sav"), b"gym 2 on PC").unwrap();
    ok(&pc_vault, &["capture", "Emerald", "--closed"]);
    fs::write(odin_save.join("emerald.sav"), b"safari zone on Odin").unwrap();
    let odin_capture = ok(&odin_vault, &["capture", "Emerald", "--closed"]);
    let odin_progress = short_id_after(&odin_capture, "Checkpoint ");
    ok(&pc_vault, &["send", "Emerald", "-o", &pkg("2.dokipkg")]);
    ok(&odin_vault, &["receive", &pkg("2.dokipkg")]);

    // Divergence is detected and both are preserved.
    let incoming = ok(&odin_vault, &["incoming"]);
    assert!(incoming.contains("You played on both devices."), "{incoming}");
    let pc_progress = incoming.split_whitespace().next().unwrap().to_owned();
    let used = ok(&odin_vault, &["use", "Emerald", &pc_progress, "--yes"]);
    assert!(used.contains("Both versions will stay in your history."));
    assert!(used.contains(&format!("Separate progress {odin_progress}")));
    assert_eq!(fs::read(odin_save.join("emerald.sav")).unwrap(), b"gym 2 on PC");

    // Restore the other one.
    let history = ok(&odin_vault, &["history", "Emerald"]);
    assert!(history.contains("separate progress"), "{history}");
    ok(&odin_vault, &["restore", "Emerald", &odin_progress, "--yes"]);
    assert_eq!(fs::read(odin_save.join("emerald.sav")).unwrap(), b"safari zone on Odin");

    // The PC learns the outcome as an ordinary continuation.
    ok(&odin_vault, &["send", "Emerald", "-o", &pkg("3.dokipkg")]);
    ok(&pc_vault, &["receive", &pkg("3.dokipkg")]);
    let incoming = ok(&pc_vault, &["incoming"]);
    assert!(incoming.contains("Continues from your current save."), "{incoming}");
    assert_eq!(fs::read(pc_save.join("emerald.sav")).unwrap(), b"gym 2 on PC", "still untouched");
}

#[test]
fn errors_say_what_is_protected() {
    let tmp = tempfile::tempdir().unwrap();
    let vault = tmp.path().join("v");
    let save = tmp.path().join("s");
    fs::create_dir_all(&save).unwrap();
    ok(&vault, &["init", "--name", "PC"]);
    ok(&vault, &["add", "Game", save.to_str().unwrap()]);
    let out = doki(&vault, &["capture", "Game", "--closed"]);
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("[BundleIncomplete]") && err.contains("Nothing was captured"), "{err}");
}

#[test]
fn hand_off_through_a_relay() {
    let tmp = tempfile::tempdir().unwrap();
    let relay = doki_relay::Relay::bind(
        doki_relay::RelayConfig::new(tmp.path().join("relay")),
        "127.0.0.1:0",
    )
    .unwrap();
    let url = format!("http://{}", relay.local_addr());
    std::thread::spawn(move || relay.run());

    let (pc, odin) = (tmp.path().join("pc"), tmp.path().join("odin"));
    let (pc_save, odin_save) = (tmp.path().join("pc-save"), tmp.path().join("odin-save"));
    fs::create_dir_all(&pc_save).unwrap();
    fs::create_dir_all(&odin_save).unwrap();
    ok(&pc, &["init", "--name", "PC"]);
    ok(&odin, &["init", "--name", "Odin"]);

    let created = ok(&pc, &["relay", "create", &url]);
    let code = created.split("doki relay join ").nth(1).unwrap().split_whitespace().next().unwrap();
    ok(&odin, &["relay", "join", code]);

    ok(&pc, &["add", "Emerald", pc_save.to_str().unwrap()]);
    fs::write(pc_save.join("emerald.sav"), b"gym 1").unwrap();
    ok(&pc, &["capture", "Emerald", "--closed"]);
    assert!(ok(&pc, &["sync"]).contains("Sent your current save of Emerald."));

    let synced = ok(&odin, &["sync"]);
    let bundle =
        synced.split("--link ").nth(1).unwrap().split('`').next().unwrap().trim().to_owned();
    ok(&odin, &["add", "Emerald", odin_save.to_str().unwrap(), "--link", &bundle]);
    let incoming = ok(&odin, &["incoming"]);
    let cp = incoming.split_whitespace().next().unwrap().to_owned();
    ok(&odin, &["use", "Emerald", &cp, "--yes"]);
    assert_eq!(fs::read(odin_save.join("emerald.sav")).unwrap(), b"gym 1");
}
