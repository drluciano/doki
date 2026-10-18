//! Mutable records the engine keeps in the vault.
//!
//! These change over time, unlike checkpoints. Each is stored as one JSON
//! file replaced atomically, keyed as noted on each type.

use std::collections::BTreeSet;

use doki_core::{BundleId, Checkpoint, CheckpointId, ContentHash, ProfileRef};
use doki_platform::member::Member;
use serde::{Deserialize, Serialize};

pub(crate) const BUNDLES: &str = "bundles";
pub(crate) const PROPOSALS: &str = "proposals";
pub(crate) const JOURNALS: &str = "journals";
pub(crate) const UNMAPPED: &str = "unmapped";
pub(crate) const WATCH: &str = "watch";
pub(crate) const PRUNED: &str = "pruned";
pub(crate) const FORGOTTEN: &str = "forgotten";
pub(crate) const SETTINGS: &str = "settings";
pub(crate) const SYSTEMS: &str = "systems";
pub(crate) const REMOTE_SYSTEMS: &str = "remote-systems";

pub const JOURNAL_FORMAT: u32 = 1;

/// A game save set up on this device. Key: bundle id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleRecord {
    pub bundle_id: BundleId,
    /// Display name, e.g. "Pokémon Emerald".
    pub name: String,
    /// Where the save folder is, in the form the location provider opens
    /// (a filesystem path for desktop folders).
    pub location: String,
    pub profile: ProfileRef,
    /// The checkpoint this device's playable files are believed to match.
    /// Only capture and a committed install change it.
    pub head: Option<CheckpointId>,
    /// Pinned checkpoints are never removed by cleanup.
    #[serde(default)]
    pub pins: BTreeSet<CheckpointId>,
    pub created_at: String,
    /// Take checkpoints on its own once the save folder has stopped
    /// changing ([`crate::Engine::auto_check`]). On unless turned off.
    #[serde(default = "yes")]
    pub auto_capture: bool,
    /// Set when the game was found inside a whole-system folder. `location`
    /// is then the system's folder, kept in step with it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member: Option<MemberRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemberKind {
    /// A subfolder of the system folder.
    Folder,
    /// Loose files in the system folder sharing a name.
    Files,
}

/// A game inside a whole-system folder: which system, and which subfolder
/// or file name in it.
///
/// The game's id is derived from these, so once a system is linked across
/// devices (an explicit choice), the same game file in it is the same game
/// on every device without any matching by title.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberRef {
    pub system_id: BundleId,
    pub kind: MemberKind,
    /// Subfolder name, or file name without extensions.
    pub name: String,
}

impl MemberRef {
    pub(crate) fn from_member(system_id: &BundleId, member: &Member) -> Self {
        let (kind, name) = match member {
            Member::Folder(n) => (MemberKind::Folder, n),
            Member::Files(n) => (MemberKind::Files, n),
        };
        MemberRef { system_id: system_id.clone(), kind, name: name.clone() }
    }

    pub(crate) fn member(&self) -> Member {
        match self.kind {
            MemberKind::Folder => Member::Folder(self.name.clone()),
            MemberKind::Files => Member::Files(self.name.clone()),
        }
    }

    /// Identifies the member within its system, e.g. `folder:Zelda`.
    pub fn key(&self) -> String {
        let kind = match self.kind {
            MemberKind::Folder => "folder",
            MemberKind::Files => "files",
        };
        format!("{kind}:{}", self.name)
    }

    /// The game id every device derives for this member.
    pub fn bundle_id(&self) -> BundleId {
        let hash = ContentHash::of_bytes(
            format!("doki-member-v1\0{}\0{}", self.system_id, self.key()).as_bytes(),
        );
        BundleId::parse(&hash.hex()[..16]).expect("hex digest is a valid id")
    }
}

/// A whole-system save folder: one emulator folder where every game's
/// subfolder or save files are tracked as their own game, including games
/// that appear later. Key: system id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemRecord {
    pub system_id: BundleId,
    /// Display name, e.g. "Eden" or "RetroArch GBA".
    pub name: String,
    pub location: String,
    pub created_at: String,
    /// Automatic checkpoints for games found from now on.
    #[serde(default = "yes")]
    pub auto_capture: bool,
    /// Member keys ([`MemberRef::key`]) the user stopped tracking; they are
    /// not added again until the user asks.
    #[serde(default)]
    pub ignored: BTreeSet<String>,
}

/// A system set up on another device and not on this one. Key: system id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteSystem {
    pub system_id: BundleId,
    pub name: String,
    pub from_device: String,
}

/// What the user decided about incoming progress.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "status")]
pub enum ProposalStatus {
    /// Waiting for review. Shown in Incoming.
    Reviewable,
    /// "Keep alongside": stays available in history, no longer prompts.
    Parked,
    /// "Dismiss": a choice checkpoint kept local progress. Never re-prompts
    /// on any device, because the choice is part of ancestry.
    Dismissed,
    /// Installed with "Use this save".
    Applied,
    /// A newer checkpoint from the same continuation arrived.
    Superseded { by: CheckpointId },
}

/// Incoming progress for one bundle. Key: `<bundle>.<checkpoint>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proposal {
    pub bundle_id: BundleId,
    pub checkpoint: CheckpointId,
    /// Name the sending device gave itself.
    pub from_device: String,
    pub received_at: String,
    #[serde(flatten)]
    pub status: ProposalStatus,
}

impl Proposal {
    pub(crate) fn key(bundle: &BundleId, cp: &CheckpointId) -> String {
        format!("{bundle}.{cp}")
    }
}

/// A bundle seen in received data that is not set up on this device yet.
/// Key: bundle id. Mapping is explicit: Doki never guesses which local game
/// an incoming save belongs to from names or paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnmappedBundle {
    pub bundle_id: BundleId,
    pub name_hint: String,
    pub from_device: String,
    /// Progress for it has arrived and waits in Incoming once it is set up.
    /// False when another device only announced the game.
    #[serde(default = "yes")]
    pub has_progress: bool,
    /// Set when the game is part of a system on the other device; it is set
    /// up here on its own once that system is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member: Option<MemberRef>,
}

fn yes() -> bool {
    true
}

/// A checkpoint whose files this device no longer keeps. Key: checkpoint id.
///
/// The checkpoint itself (its parents and file list) stays in the vault, so
/// ancestry still decides how other checkpoints relate (section 7). Only the
/// file contents are gone; the checkpoint can no longer be used or restored
/// here, and packages carry it as metadata only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrunedCheckpoint {
    pub bundle_id: BundleId,
    pub removed_at: String,
}

/// A game the user deleted from this device after removing it. Key: bundle
/// id. Doki ignores it from then on: other devices' offers and announcements
/// for it are not listed or stored here. Undone with `Engine::unforget_game`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgottenGame {
    pub bundle_id: BundleId,
    pub name: String,
    pub forgotten_at: String,
}

/// Choices that apply to the whole vault on this device. Key: `device`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    /// Remove the files of checkpoints older than this many days. `None`
    /// (the default) keeps every checkpoint. See `Engine::apply_retention`
    /// for what is always kept.
    #[serde(default)]
    pub keep_days: Option<u32>,
    /// Tell the user when an automatic checkpoint is saved.
    #[serde(default = "yes")]
    pub notify_checkpoints: bool,
    /// Tell the user when a save folder starts changing.
    #[serde(default = "yes")]
    pub notify_changes: bool,
    /// Tell the user when progress from another device arrives.
    #[serde(default = "yes")]
    pub notify_incoming: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            keep_days: None,
            notify_checkpoints: true,
            notify_changes: true,
            notify_incoming: true,
        }
    }
}

/// Why an install is running.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum InstallPurpose {
    /// Use a checkpoint (incoming progress or a restore from history).
    Use {
        checkpoint: CheckpointId,
        /// Written on commit when the current head is not an ancestor of the
        /// installed checkpoint, so the set-aside progress stays in
        /// ancestry. Stored here, not in the vault, so an abandoned install
        /// leaves no trace in history.
        choice: Option<Box<Checkpoint>>,
        /// Proposal to mark Applied on commit.
        proposal: Option<String>,
    },
    /// Recovery chose to put the pre-install save back.
    PutBack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstallPhase {
    /// Op id recorded; staging may exist; no playable file touched yet.
    Prepared,
    /// Playable files may be mixed.
    Installing,
    /// All steps done; verifying and committing.
    Committing,
}

/// Durable record of an install in progress. Key: bundle id.
///
/// Written before any staging and updated before the first destructive step,
/// so after any interruption Doki knows which two complete states the
/// playable files must end up in: `target` or `backup`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Journal {
    pub format: u32,
    pub op_id: String,
    pub bundle_id: BundleId,
    pub purpose: InstallPurpose,
    /// What is being installed. `None` means "no files" (putting back a
    /// folder that was empty before).
    pub target: Option<CheckpointId>,
    /// Verified copy of what the playable files held before. `None` means
    /// the folder had no save files.
    pub backup: Option<CheckpointId>,
    /// Head before the install, restored if recovery puts the backup back.
    pub previous_head: Option<CheckpointId>,
    pub phase: InstallPhase,
    /// Unexpected states captured during recovery, kept in history.
    #[serde(default)]
    pub rescued: Vec<CheckpointId>,
    pub started_at: String,
}
