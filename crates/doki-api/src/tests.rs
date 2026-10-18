use doki_engine::LocalProvider;
use serde_json::{json, Value};

use super::handle;

fn call(vault: &str, req: Value) -> Value {
    let out: Value = serde_json::from_str(&handle(vault, LocalProvider, &req.to_string())).unwrap();
    out
}

fn ok(vault: &str, req: Value) -> Value {
    let out = call(vault, req.clone());
    assert_eq!(out["ok"], true, "{req} -> {out}");
    out["result"].clone()
}

#[test]
fn full_flow_over_json() {
    let tmp = tempfile::tempdir().unwrap();
    let (pc, odin) = (tmp.path().join("pc"), tmp.path().join("odin"));
    let (pc, odin) = (pc.to_str().unwrap(), odin.to_str().unwrap());
    let pc_save = tmp.path().join("pc-save");
    let odin_save = tmp.path().join("odin-save");
    std::fs::create_dir_all(&pc_save).unwrap();
    std::fs::create_dir_all(&odin_save).unwrap();

    ok(pc, json!({"cmd": "init", "name": "PC"}));
    ok(odin, json!({"cmd": "init", "name": "Odin"}));
    let game = ok(pc, json!({"cmd": "add", "name": "Emerald", "location": pc_save}));
    let id = game["bundle_id"].as_str().unwrap().to_owned();
    std::fs::write(pc_save.join("e.sav"), b"one").unwrap();
    let cap = ok(pc, json!({"cmd": "capture", "game": "Emerald"}));
    assert_eq!(cap["outcome"], "created");

    // Hand off with a package file through the engine directly.
    let pkg = tmp.path().join("p.dokipkg");
    let pc_engine = doki_engine::Engine::open(pc).unwrap();
    pc_engine.export_package(&[doki_core::BundleId::parse(&id).unwrap()], &pkg).unwrap();
    doki_engine::Engine::open(odin).unwrap().import_package(&pkg).unwrap();

    assert_eq!(ok(odin, json!({"cmd": "unmapped"})).as_array().unwrap().len(), 1);
    ok(odin, json!({"cmd": "add", "name": "Emerald", "location": odin_save, "link": id}));
    let games = ok(odin, json!({"cmd": "games"}));
    assert_eq!(games[0]["incoming"], 1);
    assert_eq!(games[0]["status"], "no_checkpoint");

    let incoming = ok(odin, json!({"cmd": "incoming"}));
    let cp = incoming[0]["proposal"]["checkpoint"].as_str().unwrap().to_owned();
    let review = ok(odin, json!({"cmd": "review", "game": "Emerald", "checkpoint": &cp[..8]}));
    assert!(review["relation"].is_null());
    let used = ok(odin, json!({"cmd": "use", "review": review}));
    assert_eq!(used["installed"], cp);
    assert_eq!(std::fs::read(odin_save.join("e.sav")).unwrap(), b"one");

    let history = ok(odin, json!({"cmd": "history", "game": "Emerald"}));
    assert_eq!(history[0]["is_head"], true);
    assert!(ok(odin, json!({"cmd": "recovery_status", "game": "Emerald"})).is_null());
}

#[test]
fn errors_are_structured() {
    let tmp = tempfile::tempdir().unwrap();
    let v = tmp.path().join("v");
    let v = v.to_str().unwrap();
    let out = call(v, json!({"cmd": "games"}));
    assert_eq!(out["ok"], false);
    assert_eq!(out["error"]["code"], "NotFound");
    assert_eq!(out["error"]["save_may_be_mixed"], false);
    let out: Value = serde_json::from_str(&handle(v, LocalProvider, "not json")).unwrap();
    assert_eq!(out["error"]["code"], "InvalidInput");
    let out = call(v, json!({"cmd": "explode"}));
    assert_eq!(out["error"]["code"], "InvalidInput");
}

#[test]
fn a_tampered_review_cannot_skip_the_recheck() {
    let tmp = tempfile::tempdir().unwrap();
    let v = tmp.path().join("v");
    let v = v.to_str().unwrap();
    let save = tmp.path().join("s");
    std::fs::create_dir_all(&save).unwrap();
    ok(v, json!({"cmd": "init", "name": "PC"}));
    ok(v, json!({"cmd": "add", "name": "G", "location": save}));
    std::fs::write(save.join("g.sav"), b"old").unwrap();
    let first =
        ok(v, json!({"cmd": "capture", "game": "G"}))["checkpoint"].as_str().unwrap().to_owned();
    std::fs::write(save.join("g.sav"), b"new").unwrap();
    ok(v, json!({"cmd": "capture", "game": "G"}));
    let mut review = ok(v, json!({"cmd": "review", "game": "G", "checkpoint": first}));
    // Claim the folder was empty so nothing would need backing up.
    review["baseline_files"] = Value::Null;
    let out = call(v, json!({"cmd": "use", "review": review}));
    assert_eq!(out["error"]["code"], "LocalChangedSinceReview");
    assert_eq!(std::fs::read(save.join("g.sav")).unwrap(), b"new");
}

#[test]
fn peers_pair_and_sync_over_json() {
    // Only the peer commands read this variable.
    std::env::set_var("DOKI_PEER_NETWORK", "loopback");
    let tmp = tempfile::tempdir().unwrap();
    let (pc, odin) = (tmp.path().join("pc"), tmp.path().join("odin"));
    let (pc, odin) = (pc.to_str().unwrap(), odin.to_str().unwrap());
    let save = tmp.path().join("save");
    std::fs::create_dir_all(&save).unwrap();
    ok(pc, json!({"cmd": "init", "name": "PC"}));
    ok(odin, json!({"cmd": "init", "name": "Odin"}));
    ok(pc, json!({"cmd": "add", "name": "Emerald", "location": save}));
    std::fs::write(save.join("e.sav"), b"one").unwrap();
    ok(pc, json!({"cmd": "capture", "game": "Emerald"}));

    let code = ok(pc, json!({"cmd": "peer_invite"}))["code"].as_str().unwrap().to_owned();
    assert_eq!(ok(odin, json!({"cmd": "peer_join", "code": code}))["name"], "PC");
    assert_eq!(ok(odin, json!({"cmd": "unmapped"})).as_array().unwrap().len(), 1);
    let peers = ok(pc, json!({"cmd": "peer_list"}));
    assert_eq!(peers[0]["name"], "Odin");

    let report = ok(odin, json!({"cmd": "peer_sync"}));
    assert_eq!(report["sessions"][0]["peer_name"], "PC");
    assert_eq!(report["sessions"][0]["sent"], 0);

    let id = peers[0]["id"].as_str().unwrap();
    ok(pc, json!({"cmd": "peer_remove", "id": id}));
    let report = ok(odin, json!({"cmd": "peer_sync"}));
    assert_eq!(report["unreachable"].as_array().unwrap().len(), 1);
}

#[test]
fn edit_remove_and_auto_check_over_json() {
    let tmp = tempfile::tempdir().unwrap();
    let vault = tmp.path().join("v");
    let vault = vault.to_str().unwrap();
    let save = tmp.path().join("save");
    std::fs::create_dir_all(&save).unwrap();
    ok(vault, json!({"cmd": "init", "name": "PC"}));
    ok(vault, json!({"cmd": "add", "name": "Emerald", "location": save}));

    std::fs::write(save.join("e.sav"), b"one").unwrap();
    let first = ok(vault, json!({"cmd": "auto_check", "settle_secs": 0}));
    assert_eq!(first[0]["outcome"], "waiting");
    let second = ok(vault, json!({"cmd": "auto_check", "settle_secs": 0}));
    assert_eq!(second[0]["outcome"], "created");
    assert!(second[0]["checkpoint"].is_string());

    let edited =
        ok(vault, json!({"cmd": "edit", "game": "Emerald", "name": "Ruby", "auto_capture": false}));
    assert_eq!(
        (edited["name"].as_str(), edited["auto_capture"].as_bool()),
        (Some("Ruby"), Some(false))
    );
    assert_eq!(ok(vault, json!({"cmd": "auto_check"})), json!([]));

    ok(vault, json!({"cmd": "remove", "game": "Ruby"}));
    assert_eq!(ok(vault, json!({"cmd": "games"})), json!([]));
    assert_eq!(std::fs::read(save.join("e.sav")).unwrap(), b"one");
    assert_eq!(ok(vault, json!({"cmd": "unmapped"}))[0]["name_hint"], "Ruby");
}
