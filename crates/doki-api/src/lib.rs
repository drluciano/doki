//! JSON command interface to the engine.
//!
//! Front ends that are not Rust (the Android bridge today, the Flutter app
//! next) talk to Doki through one function: [`handle`] takes a JSON request
//! and returns a JSON response. Keeping the boundary this narrow means every
//! safety decision stays in the engine, never in UI code.
//!
//! # Requests
//!
//! Every request is an object with a `cmd` field. `game` accepts a game id,
//! id prefix or exact name; `checkpoint` accepts a unique id prefix.
//!
//! | `cmd` | Fields | Result |
//! |---|---|---|
//! | `init` | `name` | `{device_id, device_name}` |
//! | `device` | – | `{device_id, device_name}` |
//! | `games` | – | `[GameSummary]` |
//! | `unmapped` | – | `[UnmappedBundle]` games on other devices not set up here; `has_progress` says whether progress already arrived |
//! | `add` | `name`, `location`, `link?` | `BundleRecord` |
//! | `capture` | `game`, `note?` (caller confirms the game is closed) | `{outcome, checkpoint}` |
//! | `edit` | `game`, `name?`, `location?`, `auto_capture?` | `BundleRecord`; never touches save files |
//! | `remove` | `game` | `null`; this device only, history and save folder kept |
//! | `auto_check` | `settle_secs?` (default 30) | `[{bundle_id, name, outcome, …}]`, see `Engine::auto_check` |
//! | `history` | `game` | `[HistoryEntry]` |
//! | `incoming` | – | `[ReviewItem]` |
//! | `review` | `game`, `checkpoint` | `Review` (pass back unchanged to `use`) |
//! | `use` | `review` | `UseOutcome` |
//! | `keep`, `reopen`, `dismiss` | `game`, `checkpoint` | `null` / choice id |
//! | `recovery_status` | `game` | `RecoveryStatus` or `null` |
//! | `recover` | `game`, `action`: `finish` or `put_back` | `RecoveryOutcome` |
//! | `pin`, `unpin` | `game`, `checkpoint` | `null` |
//! | `delete_checkpoint` | `game`, `checkpoint` | `{checkpoints, bytes}`; this device only, files only (history keeps its place) |
//! | `settings` | – | `Settings` (`keep_days`, `notify_checkpoints`, `notify_changes`, `notify_incoming`) |
//! | `set_settings` | any `Settings` field; `keep_days: null` keeps forever | `Settings`; never removes anything by itself |
//! | `retention_preview` | `keep_days` | `{checkpoints, bytes}` that keeping `keep_days` days would remove |
//! | `apply_retention` | – | `{checkpoints, bytes}` removed now under the chosen time |
//! | `forget` | `game` (bundle id of a removed game) | `{checkpoints, bytes}`; deletes it from this device, other devices keep theirs |
//! | `forgotten` | – | `[ForgottenGame]` |
//! | `unforget` | `game` (bundle id) | `null`; it comes back with the next sync |
//! | `relay_create` | `url` | `{pairing_code}` |
//! | `relay_join` | `code` | `null` |
//! | `relay_code` | – | `{pairing_code}` or `null` |
//! | `relay_forget` | – | `null` |
//! | `sync` | – | `{uploaded, received, rejected}` |
//! | `peer_invite` | – | `{code}` pairing code valid 15 minutes |
//! | `peer_join` | `code` | `{name}` of the inviting device; syncs with it at once |
//! | `peer_sync` | – | `{sessions: [{peer_name, sent, received}], unreachable: [[name, reason]]}` |
//! | `peer_list` | – | `[{id, name, introduced_by, last_sync}]` |
//! | `peer_remove` | `id` | `null` |
//!
//! The `peer_*` commands use peer-to-peer sync (`doki-peer`). The first one
//! starts this device's peer node, which keeps answering other devices in
//! the background for the life of the process.
//!
//! # Responses
//!
//! `{"ok": true, "result": …}` or
//! `{"ok": false, "error": {"code", "message", "save_may_be_mixed"}}` where
//! `code` is a stable identifier from the safety contract, section 11.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

/// How long a save folder must stay unchanged before `auto_check` captures it.
const AUTO_SETTLE_SECS: u64 = 30;

use doki_core::{BundleId, CaptureMode, CheckpointId, Error, Result};
use doki_engine::{
    BundleChanges, Engine, LocationProvider, RecoveryAction, Review, SettingsChanges,
};
use doki_peer::{Network, PeerNode};
use doki_vault::Vault;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
enum Request {
    Init {
        name: String,
    },
    Device,
    Games,
    Unmapped,
    Add {
        name: String,
        location: String,
        link: Option<String>,
    },
    Capture {
        game: String,
        note: Option<String>,
    },
    Edit {
        game: String,
        name: Option<String>,
        location: Option<String>,
        auto_capture: Option<bool>,
    },
    Remove {
        game: String,
    },
    AutoCheck {
        settle_secs: Option<u64>,
    },
    History {
        game: String,
    },
    Incoming,
    Review {
        game: String,
        checkpoint: String,
    },
    Use {
        review: Box<Review>,
    },
    Keep {
        game: String,
        checkpoint: String,
    },
    Reopen {
        game: String,
        checkpoint: String,
    },
    Dismiss {
        game: String,
        checkpoint: String,
    },
    RecoveryStatus {
        game: String,
    },
    Recover {
        game: String,
        action: Action,
    },
    Pin {
        game: String,
        checkpoint: String,
    },
    Unpin {
        game: String,
        checkpoint: String,
    },
    DeleteCheckpoint {
        game: String,
        checkpoint: String,
    },
    Settings,
    SetSettings(SettingsChanges),
    RetentionPreview {
        keep_days: u32,
    },
    ApplyRetention,
    Forget {
        game: String,
    },
    Forgotten,
    Unforget {
        game: String,
    },
    RelayCreate {
        url: String,
    },
    RelayJoin {
        code: String,
    },
    RelayCode,
    RelayForget,
    Sync,
    PeerInvite,
    PeerJoin {
        code: String,
    },
    PeerSync,
    PeerList,
    PeerRemove {
        id: String,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Action {
    Finish,
    PutBack,
}

/// Run one request against the vault at `vault_path`, opening save
/// locations through `provider`. Never panics on bad input.
pub fn handle<P: LocationProvider>(vault_path: &str, provider: P, request: &str) -> String {
    let response = match serde_json::from_str::<Request>(request) {
        Err(e) => Err(Error::InvalidInput(format!("bad request: {e}"))),
        Ok(req) => dispatch(vault_path, provider, req),
    };
    match response {
        Ok(result) => json!({ "ok": true, "result": result }).to_string(),
        Err(e) => json!({
            "ok": false,
            "error": {
                "code": e.code(),
                "message": e.to_string(),
                "save_may_be_mixed": e.playable_files_may_be_mixed(),
            }
        })
        .to_string(),
    }
}

/// The running peer node for `vault_path`, started on first use.
///
/// `DOKI_PEER_NETWORK=loopback` keeps it on this machine (tests only).
fn peer_node(vault_path: &str) -> Result<Arc<PeerNode>> {
    static NODES: OnceLock<Mutex<HashMap<String, Arc<PeerNode>>>> = OnceLock::new();
    let mut nodes = NODES.get_or_init(Default::default).lock().unwrap_or_else(|p| p.into_inner());
    if let Some(node) = nodes.get(vault_path) {
        return Ok(node.clone());
    }
    let network = match std::env::var("DOKI_PEER_NETWORK").as_deref() {
        Ok("loopback") => Network::Loopback,
        _ => Network::Internet,
    };
    let node = Arc::new(PeerNode::start(vault_path, network)?);
    nodes.insert(vault_path.to_owned(), node.clone());
    Ok(node)
}

fn to_value<T: serde::Serialize>(v: T) -> Result<Value> {
    Ok(serde_json::to_value(v).expect("engine types serialize"))
}

fn dispatch<P: LocationProvider>(vault_path: &str, provider: P, req: Request) -> Result<Value> {
    if let Request::Init { name } = &req {
        let v = Vault::init(vault_path, name)?;
        return Ok(json!({ "device_id": v.device_id(), "device_name": v.device_name() }));
    }
    let engine = Engine::with_provider(Vault::open(vault_path)?, provider);
    let game = |q: &str| engine.find_bundle(q).map(|b| b.bundle_id);
    let cp = |g: &BundleId, q: &str| -> Result<CheckpointId> { engine.find_checkpoint(g, q) };
    match req {
        Request::Init { .. } => unreachable!(),
        Request::Device => Ok(json!({
            "device_id": engine.vault().device_id(),
            "device_name": engine.vault().device_name(),
        })),
        Request::Games => {
            let incoming = engine.incoming()?;
            let mut out = Vec::new();
            for b in engine.bundles()? {
                let status = if engine.journal(&b.bundle_id)?.is_some() {
                    "needs_recovery".to_owned()
                } else {
                    match engine.has_uncaptured_changes(&b.bundle_id) {
                        Err(e) => format!("unavailable:{}", e.code()),
                        Ok(true) => "uncaptured_changes".into(),
                        Ok(false) if b.head.is_some() => "protected".into(),
                        Ok(false) => "no_checkpoint".into(),
                    }
                };
                let waiting =
                    incoming.iter().filter(|i| i.proposal.bundle_id == b.bundle_id).count();
                out.push(json!({ "game": b, "status": status, "incoming": waiting }));
            }
            Ok(Value::Array(out))
        }
        Request::Unmapped => to_value(engine.unmapped_bundles()?),
        Request::Add { name, location, link } => {
            let link = link.map(|l| BundleId::parse(&l)).transpose()?;
            to_value(engine.add_bundle(&name, &location, link)?)
        }
        Request::Capture { game: g, note } => {
            to_value(engine.capture(&game(&g)?, CaptureMode::EmulatorStoppedConfirmed, note)?)
        }
        Request::Edit { game: g, name, location, auto_capture } => to_value(
            engine.update_bundle(&game(&g)?, BundleChanges { name, location, auto_capture })?,
        ),
        Request::Remove { game: g } => {
            engine.remove_bundle(&game(&g)?)?;
            Ok(Value::Null)
        }
        Request::AutoCheck { settle_secs } => to_value(
            engine.auto_check(Duration::from_secs(settle_secs.unwrap_or(AUTO_SETTLE_SECS)))?,
        ),
        Request::History { game: g } => to_value(engine.history(&game(&g)?)?),
        Request::Incoming => to_value(engine.incoming()?),
        Request::Review { game: g, checkpoint } => {
            let g = game(&g)?;
            to_value(engine.review(&g, &cp(&g, &checkpoint)?)?)
        }
        Request::Use { review } => to_value(engine.use_checkpoint(&review)?),
        Request::Keep { game: g, checkpoint } => {
            let g = game(&g)?;
            engine.keep_alongside(&g, &cp(&g, &checkpoint)?)?;
            Ok(Value::Null)
        }
        Request::Reopen { game: g, checkpoint } => {
            let g = game(&g)?;
            engine.reopen(&g, &cp(&g, &checkpoint)?)?;
            Ok(Value::Null)
        }
        Request::Dismiss { game: g, checkpoint } => {
            let g = game(&g)?;
            to_value(engine.dismiss(&g, &cp(&g, &checkpoint)?)?)
        }
        Request::RecoveryStatus { game: g } => to_value(engine.recovery_status(&game(&g)?)?),
        Request::Recover { game: g, action } => {
            let action = match action {
                Action::Finish => RecoveryAction::Finish,
                Action::PutBack => RecoveryAction::PutBack,
            };
            to_value(engine.recover(&game(&g)?, action)?)
        }
        Request::Pin { game: g, checkpoint } => {
            let g = game(&g)?;
            engine.set_pinned(&g, &cp(&g, &checkpoint)?, true)?;
            Ok(Value::Null)
        }
        Request::Unpin { game: g, checkpoint } => {
            let g = game(&g)?;
            engine.set_pinned(&g, &cp(&g, &checkpoint)?, false)?;
            Ok(Value::Null)
        }
        Request::DeleteCheckpoint { game: g, checkpoint } => {
            let g = game(&g)?;
            to_value(engine.delete_checkpoint(&g, &cp(&g, &checkpoint)?)?)
        }
        Request::Settings => to_value(engine.settings()?),
        Request::SetSettings(changes) => to_value(engine.update_settings(changes)?),
        Request::RetentionPreview { keep_days } => to_value(engine.retention_preview(keep_days)?),
        Request::ApplyRetention => to_value(engine.apply_retention()?),
        Request::Forget { game: g } => to_value(engine.forget_game(&BundleId::parse(&g)?)?),
        Request::Forgotten => to_value(engine.forgotten_games()?),
        Request::Unforget { game: g } => {
            engine.unforget_game(&BundleId::parse(&g)?)?;
            Ok(Value::Null)
        }
        Request::RelayCreate { url } => {
            if doki_sync::load_link(&engine)?.is_some() {
                return Err(Error::InvalidInput("already paired with a relay".into()));
            }
            let link = doki_sync::RelayLink::create(&url)?;
            doki_sync::connect(&link)?;
            doki_sync::save_link(&engine, &link)?;
            Ok(json!({ "pairing_code": link.pairing_code() }))
        }
        Request::RelayJoin { code } => {
            if doki_sync::load_link(&engine)?.is_some() {
                return Err(Error::InvalidInput("already paired with a relay".into()));
            }
            let link = doki_sync::RelayLink::from_pairing_code(&code)?;
            doki_sync::connect(&link)?;
            doki_sync::save_link(&engine, &link)?;
            Ok(Value::Null)
        }
        Request::RelayCode => Ok(doki_sync::load_link(&engine)?
            .map(|l| json!({ "pairing_code": l.pairing_code() }))
            .unwrap_or(Value::Null)),
        Request::RelayForget => {
            doki_sync::forget_link(&engine)?;
            Ok(Value::Null)
        }
        Request::Sync => {
            let r = doki_sync::sync(&engine)?;
            Ok(json!({ "uploaded": r.uploaded, "received": r.received, "rejected": r.rejected }))
        }
        Request::PeerInvite => to_value(peer_node(vault_path)?.invite()?),
        Request::PeerJoin { code } => Ok(json!({ "name": peer_node(vault_path)?.join(&code)? })),
        Request::PeerSync => to_value(peer_node(vault_path)?.sync_all()?),
        Request::PeerList => to_value(peer_node(vault_path)?.peers()?),
        Request::PeerRemove { id } => {
            peer_node(vault_path)?.remove(&id)?;
            Ok(Value::Null)
        }
    }
}

#[cfg(test)]
mod tests;
