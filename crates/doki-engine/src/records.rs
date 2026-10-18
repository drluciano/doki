//! Mutable records the engine keeps in the vault.
//!
//! These change over time, unlike checkpoints. Each is stored as one JSON
//! file replaced atomically, keyed as noted on each type.

use std::collections::BTreeSet;

use doki_core::{BundleId, Checkpoint, CheckpointId, ProfileRef};
use serde::{Deserialize, Serialize};

pub(crate) const BUNDLES: &str = "bundles";
pub(crate) const PROPOSALS: &str = "proposals";
pub(crate) const JOURNALS: &str = "journals";
pub(crate) const UNMAPPED: &str = "unmapped";

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
}

fn yes() -> bool {
    true
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
