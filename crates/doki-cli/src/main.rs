//! `doki`: command-line front end for the safety engine.
//!
//! Uses the same words as the app: Checkpoint, Separate progress, Incoming
//! progress, Use this save, Restore a checkpoint. See `docs/cli.md`.

use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use doki_core::{BundleId, CaptureMode, Checkpoint, CheckpointKind, Error, Relation};
use doki_engine::{CaptureOutcome, Engine, FileState, RecoveryAction, RecoveryOutcome, Review};
use doki_vault::Vault;

#[derive(Parser)]
#[command(name = "doki", version, about = "Hand saves between devices without losing progress")]
struct Cli {
    /// Vault folder. Defaults to $DOKI_VAULT, else the user data folder.
    #[arg(long, global = true, env = "DOKI_VAULT")]
    vault: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create a vault for this device.
    Init {
        /// Name other devices will see, e.g. "PC" or "Odin".
        #[arg(long)]
        name: String,
    },
    /// Set up a game's save folder. Reads nothing and changes nothing yet.
    Add {
        name: String,
        folder: PathBuf,
        /// Link to a game already set up on another device (its id is shown
        /// by `doki games` after receiving its progress).
        #[arg(long)]
        link: Option<String>,
    },
    /// List games, their state and anything waiting for you.
    Games,
    /// Capture the current save as a checkpoint. Close the game first.
    Capture {
        game: String,
        /// Confirm the game is closed (otherwise you are asked).
        #[arg(long)]
        closed: bool,
        #[arg(long)]
        note: Option<String>,
    },
    /// Show a game's checkpoints, oldest first.
    History {
        game: String,
    },
    /// Write a handoff file with your current save of each game.
    Send {
        #[arg(required = true)]
        games: Vec<String>,
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Receive a handoff file. This never changes your saves.
    Receive {
        file: PathBuf,
    },
    /// List incoming progress waiting for review.
    Incoming,
    /// Review incoming progress or a checkpoint, then choose to use it.
    Use {
        game: String,
        checkpoint: String,
        /// Skip the confirmation question.
        #[arg(long)]
        yes: bool,
    },
    /// Restore a checkpoint from history (same safety steps as Use).
    Restore {
        game: String,
        checkpoint: String,
        #[arg(long)]
        yes: bool,
    },
    /// Keep incoming progress alongside yours without using it.
    Keep {
        game: String,
        checkpoint: String,
    },
    /// Dismiss incoming progress and keep your own. It stays in history.
    Dismiss {
        game: String,
        checkpoint: String,
    },
    /// Show or resolve an interrupted install.
    Recover {
        game: String,
        /// Finish installing the save you chose.
        #[arg(long, conflicts_with = "put_back")]
        finish: bool,
        /// Put back the save you had before.
        #[arg(long)]
        put_back: bool,
    },
    /// Protect a checkpoint from any future cleanup.
    Pin {
        game: String,
        checkpoint: String,
    },
    Unpin {
        game: String,
        checkpoint: String,
    },
    /// Copy a checkpoint's files to a new folder, outside Doki.
    ExportFiles {
        game: String,
        checkpoint: String,
        folder: PathBuf,
    },
    /// Remove temporary leftovers. Never removes checkpoints.
    Clean,
    /// Pair this device with a relay so devices can exchange progress
    /// without being online together.
    #[command(subcommand)]
    Relay(RelayCommand),
    /// Send your current saves to the relay and receive other devices'
    /// progress. Never changes your saves; review incoming progress after.
    Sync,
}

#[derive(Subcommand)]
enum RelayCommand {
    /// Start a new space for your devices on a relay and pair this device.
    Create { url: String },
    /// Pair this device using the code shown on your first device.
    Join { code: String },
    /// Show the pairing code for adding another device.
    Code,
    /// Unpair this device. Nothing is deleted from the relay or other devices.
    Forget,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}", explain(&e));
            ExitCode::from(if e.playable_files_may_be_mixed() { 3 } else { 1 })
        }
    }
}

/// What happened, whether the save changed, and what to do next (I11).
fn explain(e: &Error) -> String {
    let next = match e {
        Error::SourceUnavailable(_) => "Your save was not changed. Reconnect the folder or storage and try again.",
        Error::PermissionLost(_) => "Your save was not changed. Allow access to the folder again.",
        Error::SourceChanging => "Your save was not changed. Close the game, then try again.",
        Error::BundleIncomplete(_) => "Nothing was captured. Check that the folder holds the save.",
        Error::StorageFull { .. } => "Your save was not changed. Free some space and try again.",
        Error::IntegrityFailed(_) => "Nothing was installed. Receive the progress again; report it if this repeats.",
        Error::LocalChangedSinceReview { .. } => {
            "Your save was not changed. Your newer progress was saved as a checkpoint; review again."
        }
        Error::BackupUnverified(_) => "Your save was not changed, because it could not be backed up safely.",
        Error::StrategyUnsupported(_) => "Your save was not changed.",
        Error::InstallInterrupted(_) | Error::RecoveryRequired => {
            "Your previous save is safe in history. Run `doki recover <game>` before playing."
        }
        Error::RecoveryFailed(_) => {
            "Do not play this game yet. Your previous save is safe in history; use `doki export-files` to copy it out, then retry `doki recover`."
        }
        Error::Busy => "Wait for the other Doki operation on this game to finish.",
        _ => "",
    };
    format!("Error [{}]: {e}\n{next}", e.code()).trim_end().to_owned()
}

fn default_vault() -> PathBuf {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return PathBuf::from(appdata).join("Doki");
    }
    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(xdg).join("doki");
    }
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    home.join(".local/share/doki")
}

fn confirm(question: &str) -> io::Result<bool> {
    print!("{question} [y/N] ");
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line)?;
    Ok(matches!(line.trim().to_lowercase().as_str(), "y" | "yes"))
}

fn when(cp: &Checkpoint) -> String {
    cp.captured_at.replace('T', " ").replace('Z', " UTC")
}

fn kind_label(cp: &Checkpoint) -> &'static str {
    match cp.kind {
        CheckpointKind::Capture => "Checkpoint",
        CheckpointKind::Backup => "Backup",
        CheckpointKind::Choice => "Choice",
    }
}

fn relation_label(r: &Option<Relation>) -> &'static str {
    match r {
        None => "This device has no save of this game yet.",
        Some(Relation::Same) => "This is the save you are using.",
        Some(Relation::Ahead) => "Continues from your current save.",
        Some(Relation::Behind) => "Older than your current save.",
        Some(Relation::Divergent { .. }) => "You played on both devices.",
        Some(Relation::Unrelated) => "These saves do not share history.",
        Some(Relation::Unknown { .. }) => "Some history is missing; the relation is unknown.",
    }
}

fn run(cli: Cli) -> Result<(), Error> {
    let vault_dir = cli.vault.unwrap_or_else(default_vault);
    if let Command::Init { name } = &cli.command {
        let v = Vault::init(&vault_dir, name)?;
        println!("Created vault for \"{}\" at {}", v.device_name(), vault_dir.display());
        return Ok(());
    }
    let engine = Engine::open(&vault_dir)?;
    let device_name = |cp: &Checkpoint| -> String {
        if &cp.origin_device == engine.vault().device_id() {
            "this device".into()
        } else {
            cp.origin_device_name.clone()
        }
    };

    match cli.command {
        Command::Init { .. } => unreachable!(),
        Command::Add { name, folder, link } => {
            let link = link.map(|l| BundleId::parse(&l)).transpose()?;
            let b = engine.add_bundle(&name, &folder.to_string_lossy(), link)?;
            println!("Set up {} ({}) at {}", b.name, b.bundle_id.short(), b.location);
            println!("Next: close the game and run `doki capture \"{}\"`.", b.name);
        }
        Command::Games => {
            let games = engine.bundles()?;
            if games.is_empty() {
                println!("No games set up yet. Add one with `doki add <name> <folder>`.");
            }
            let incoming = engine.incoming()?;
            for g in games {
                let status = if engine.journal(&g.bundle_id)?.is_some() {
                    "Needs recovery: run `doki recover`".to_owned()
                } else {
                    match engine.has_uncaptured_changes(&g.bundle_id) {
                        Err(e) => format!("Save location unavailable ({})", e.code()),
                        Ok(true) => "Has progress not captured yet".to_owned(),
                        Ok(false) if g.head.is_some() => "Protected locally".to_owned(),
                        Ok(false) => "No checkpoint yet".to_owned(),
                    }
                };
                let waiting =
                    incoming.iter().filter(|i| i.proposal.bundle_id == g.bundle_id).count();
                println!("{}  [{}]", g.name, g.bundle_id);
                println!("  {status}");
                if waiting > 0 {
                    println!("  Incoming progress: {waiting} (see `doki incoming`)");
                }
            }
            for u in engine.unmapped_bundles()? {
                println!(
                    "Not set up here: \"{}\" from {}. Link it with `doki add \"{}\" <folder> --link {}`",
                    u.name_hint, u.from_device, u.name_hint, u.bundle_id
                );
            }
        }
        Command::Capture { game, closed, note } => {
            let g = engine.find_bundle(&game)?;
            if !closed
                && !confirm(&format!("Is {} closed in the emulator?", g.name)).unwrap_or(false)
            {
                println!("Close the game first, so the save is complete. Nothing was captured.");
                return Ok(());
            }
            match engine.capture(&g.bundle_id, CaptureMode::EmulatorStoppedConfirmed, note)? {
                CaptureOutcome::Created(id) => {
                    let cp = engine.vault().checkpoint(&id)?;
                    println!(
                        "Checkpoint {} saved: {} · {} file(s), {} bytes, verified copy.",
                        id.short(),
                        g.name,
                        cp.files.len(),
                        cp.total_size()
                    );
                }
                CaptureOutcome::Unchanged(id) => {
                    println!("No changes since checkpoint {}.", id.short())
                }
            }
        }
        Command::History { game } => {
            let g = engine.find_bundle(&game)?;
            let history = engine.history(&g.bundle_id)?;
            if history.is_empty() {
                println!("No checkpoints yet.");
            }
            for e in history {
                let mut tags = Vec::new();
                if e.is_head {
                    tags.push("current save");
                } else if e.is_tip || e.set_aside {
                    tags.push("separate progress");
                }
                if e.pinned {
                    tags.push("pinned");
                }
                let tags =
                    if tags.is_empty() { String::new() } else { format!(" ({})", tags.join(", ")) };
                println!(
                    "{}  {} · {} · {}{}",
                    e.id.short(),
                    kind_label(&e.checkpoint),
                    device_name(&e.checkpoint),
                    when(&e.checkpoint),
                    tags
                );
                if let Some(note) = &e.checkpoint.note {
                    println!("          {note}");
                }
            }
        }
        Command::Send { games, output } => {
            let ids = games
                .iter()
                .map(|g| engine.find_bundle(g).map(|b| b.bundle_id))
                .collect::<Result<Vec<_>, _>>()?;
            let n = engine.export_package(&ids, &output)?;
            println!("Wrote {} with {n} checkpoint(s). Open it on your other device with `doki receive`.", output.display());
        }
        Command::Receive { file } => {
            let s = engine.import_package(&file)?;
            println!(
                "Received {} checkpoint(s) from {}. Your saves were not changed.",
                s.checkpoints_added, s.from_device
            );
            for (bundle, cp) in &s.new_proposals {
                let name = engine.bundle(bundle)?.name;
                println!(
                    "Progress from {} is ready for {name}: `doki use \"{name}\" {}`",
                    s.from_device,
                    cp.short()
                );
            }
            for (bundle, name) in &s.unmapped {
                println!("\"{name}\" is not set up here. Link it: `doki add \"{name}\" <folder> --link {bundle}`");
            }
        }
        Command::Incoming => {
            let items = engine.incoming()?;
            if items.is_empty() {
                println!("Nothing waiting for review.");
            }
            for i in items {
                let name = engine.bundle(&i.proposal.bundle_id)?.name;
                println!(
                    "{}  {name} · from {} · {}",
                    i.proposal.checkpoint.short(),
                    i.proposal.from_device,
                    when(&i.checkpoint)
                );
                println!("          {}", relation_label(&i.relation));
            }
        }
        Command::Use { game, checkpoint, yes } | Command::Restore { game, checkpoint, yes } => {
            let g = engine.find_bundle(&game)?;
            let id = engine.find_checkpoint(&g.bundle_id, &checkpoint)?;
            let review = engine.review(&g.bundle_id, &id)?;
            print_review(&engine, &g.name, &review)?;
            if !yes && !confirm("Back up your current save and use this one?").unwrap_or(false) {
                println!("Nothing was changed.");
                return Ok(());
            }
            let out = engine.use_checkpoint(&review)?;
            println!("Done. {} now uses checkpoint {}.", g.name, out.installed.short());
            if let Some(b) = out.backup {
                println!("Your previous save is checkpoint {} (restore it any time).", b.short());
            }
            if let Some(s) = out.set_aside {
                println!("Separate progress {} stays in your history.", s.short());
            }
        }
        Command::Keep { game, checkpoint } => {
            let g = engine.find_bundle(&game)?;
            let id = engine.find_checkpoint(&g.bundle_id, &checkpoint)?;
            engine.keep_alongside(&g.bundle_id, &id)?;
            println!("Kept alongside. It stays in history; your save was not changed.");
        }
        Command::Dismiss { game, checkpoint } => {
            let g = engine.find_bundle(&game)?;
            let id = engine.find_checkpoint(&g.bundle_id, &checkpoint)?;
            engine.dismiss(&g.bundle_id, &id)?;
            println!("Dismissed. You kept your save; the other progress stays in history.");
        }
        Command::Recover { game, finish, put_back } => {
            let g = engine.find_bundle(&game)?;
            let Some(status) = engine.recovery_status(&g.bundle_id)? else {
                println!("Nothing to recover for {}.", g.name);
                return Ok(());
            };
            if !finish && !put_back {
                println!("Installing a save for {} was interrupted.", g.name);
                for (path, state) in &status.files {
                    let s = match state {
                        FileState::Target => "new save",
                        FileState::Backup => "previous save",
                        FileState::Other => "changed by something else (will be kept in history)",
                    };
                    println!("  {path}: {s}");
                }
                if status.complete {
                    println!(
                        "All files are already in place. Run `doki recover \"{}\" --finish`.",
                        g.name
                    );
                } else {
                    println!("Choose: `doki recover \"{0}\" --finish` or `doki recover \"{0}\" --put-back`.", g.name);
                }
                return Ok(());
            }
            let action = if finish { RecoveryAction::Finish } else { RecoveryAction::PutBack };
            match engine.recover(&g.bundle_id, action)? {
                RecoveryOutcome::NothingChanged => {
                    println!("The install had not started. Your save was not changed.")
                }
                RecoveryOutcome::Finished { head } => {
                    println!("Finished installing. Current save: {}.", head.short())
                }
                RecoveryOutcome::PutBack { .. } => println!("Your previous save is back in place."),
            }
        }
        Command::Pin { game, checkpoint } => set_pin(&engine, &game, &checkpoint, true)?,
        Command::Unpin { game, checkpoint } => set_pin(&engine, &game, &checkpoint, false)?,
        Command::ExportFiles { game, checkpoint, folder } => {
            let g = engine.find_bundle(&game)?;
            let id = engine.find_checkpoint(&g.bundle_id, &checkpoint)?;
            engine.export_checkpoint_files(&id, &folder)?;
            println!("Copied checkpoint {} to {}.", id.short(), folder.display());
        }
        Command::Clean => {
            let freed = engine.clean_leftovers()?;
            println!("Removed {freed} bytes of leftovers. No checkpoints were removed.");
        }
        Command::Relay(cmd) => relay(&engine, cmd)?,
        Command::Sync => {
            let report = doki_sync::sync(&engine)?;
            for b in &report.uploaded {
                println!("Sent your current save of {}.", engine.bundle(b)?.name);
            }
            for s in &report.received {
                for (bundle, cp) in &s.new_proposals {
                    let name = engine.bundle(bundle)?.name;
                    println!(
                        "Progress from {} is ready for {name}: `doki use \"{name}\" {}`",
                        s.from_device,
                        cp.short()
                    );
                }
                for (bundle, name) in &s.unmapped {
                    println!(
                        "\"{name}\" from {} is not set up here. Link it: `doki add \"{name}\" <folder> --link {bundle}`",
                        s.from_device
                    );
                }
            }
            for (seq, why) in &report.rejected {
                eprintln!("Skipped relay package {seq}: {why}");
            }
            if report.uploaded.is_empty() && report.received.is_empty() {
                println!("Up to date. Your saves were not changed.");
            } else {
                println!("Done. Your saves were not changed.");
            }
        }
    }
    Ok(())
}

fn relay(engine: &Engine, cmd: RelayCommand) -> Result<(), Error> {
    match cmd {
        RelayCommand::Create { url } => {
            if doki_sync::load_link(engine)?.is_some() {
                return Err(Error::InvalidInput(
                    "already paired; run `doki relay forget` first".into(),
                ));
            }
            let link = doki_sync::RelayLink::create(&url)?;
            doki_sync::connect(&link)?;
            doki_sync::save_link(engine, &link)?;
            println!("Paired with {}. On your other device, run:", link.url);
            println!("  doki relay join {}", link.pairing_code());
            println!("Anyone with this code can read your progress. Share it only with your own devices.");
        }
        RelayCommand::Join { code } => {
            if doki_sync::load_link(engine)?.is_some() {
                return Err(Error::InvalidInput(
                    "already paired; run `doki relay forget` first".into(),
                ));
            }
            let link = doki_sync::RelayLink::from_pairing_code(&code)?;
            doki_sync::connect(&link)?;
            doki_sync::save_link(engine, &link)?;
            println!("Paired with {}. Run `doki sync` to exchange progress.", link.url);
        }
        RelayCommand::Code => match doki_sync::load_link(engine)? {
            Some(link) => println!("{}", link.pairing_code()),
            None => {
                println!("Not paired. Use `doki relay create <url>` or `doki relay join <code>`.")
            }
        },
        RelayCommand::Forget => {
            doki_sync::forget_link(engine)?;
            println!("Unpaired. Nothing was deleted from the relay or your other devices.");
        }
    }
    Ok(())
}

fn set_pin(engine: &Engine, game: &str, checkpoint: &str, pin: bool) -> Result<(), Error> {
    let g = engine.find_bundle(game)?;
    let id = engine.find_checkpoint(&g.bundle_id, checkpoint)?;
    engine.set_pinned(&g.bundle_id, &id, pin)?;
    println!("{} checkpoint {}.", if pin { "Pinned" } else { "Unpinned" }, id.short());
    Ok(())
}

fn print_review(engine: &Engine, game: &str, r: &Review) -> Result<(), Error> {
    let cp = &r.incoming_checkpoint;
    println!("{game}: checkpoint {}", r.incoming.short());
    println!(
        "  From: {} · {}",
        if &cp.origin_device == engine.vault().device_id() {
            "this device"
        } else {
            &cp.origin_device_name
        },
        when(cp)
    );
    println!(
        "  {} file(s), {} bytes, bytes verified (game validity not checked)",
        cp.files.len(),
        cp.total_size()
    );
    if let Some(note) = &cp.note {
        println!("  Note: {note}");
    }
    println!("  {}", relation_label(&r.relation));
    if let Some(Relation::Divergent { common }) = &r.relation {
        let names: Vec<_> = common.iter().map(|c| c.short()).collect();
        println!(
            "  Both continued from checkpoint {}. Both versions will stay in your history.",
            names.join(", ")
        );
    }
    if let Some(saved) = &r.captured_local_changes {
        println!("  Your uncaptured progress was saved first as checkpoint {}.", saved.short());
    }
    match &r.baseline_head {
        Some(h) if r.baseline_files.is_some() => {
            println!(
                "  Your current save (checkpoint {}) will be backed up and verified first.",
                h.short()
            )
        }
        _ => println!("  The save folder is empty, so nothing will be replaced."),
    }
    Ok(())
}
