//! Stable error taxonomy (safety contract section 11).
//!
//! Every error answers: what happened, could playable files have changed, and
//! what can the user do next. `code()` is stable and safe to show or log;
//! `Display` text may contain paths and is for local display only.

use std::io;

use crate::model::CheckpointId;

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Which side of an operation ran out of space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageSide {
    Vault,
    SaveLocation,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The save folder is missing, unmounted or otherwise unreachable.
    /// Never interpreted as deletion (I5).
    #[error("save location unavailable: {0}")]
    SourceUnavailable(String),

    /// The platform refused access to the save folder.
    #[error("access to the save location was denied: {0}")]
    PermissionLost(String),

    /// Files changed while being read; the emulator may still be writing.
    #[error("the save changed while it was being read; close the game and try again")]
    SourceChanging,

    /// The bundle is empty or lacks files the profile requires.
    #[error("the save is incomplete: {0}")]
    BundleIncomplete(String),

    #[error("not enough storage space in the {side:?}: {detail}")]
    StorageFull { side: StorageSide, detail: String },

    /// Stored or received bytes do not match their recorded hash or size.
    #[error("integrity check failed: {0}")]
    IntegrityFailed(String),

    #[error("this save is not compatible: {0}")]
    IncompatibleSave(String),

    /// The playable files or head changed after the user reviewed. A new
    /// checkpoint holds the changed state when one could be captured.
    #[error("the local save changed since you reviewed it; review again")]
    LocalChangedSinceReview { new_checkpoint: Option<CheckpointId> },

    /// The pre-apply backup could not be verified; nothing was replaced.
    #[error("the backup of the current save could not be verified: {0}")]
    BackupUnverified(String),

    /// The save location cannot perform the operation safely.
    #[error("this save location cannot do that safely: {0}")]
    StrategyUnsupported(String),

    /// An installation stopped partway. Playable files may be mixed until
    /// recovery finishes or puts the previous save back.
    #[error("installing was interrupted; recovery is needed: {0}")]
    InstallInterrupted(String),

    /// An earlier installation on this bundle has not been recovered yet.
    #[error("an interrupted installation must be recovered first")]
    RecoveryRequired,

    #[error("recovery could not complete: {0}")]
    RecoveryFailed(String),

    /// Another capture or install holds this bundle (I10).
    #[error("another operation is already running for this save")]
    Busy,

    #[error("not found: {0}")]
    NotFound(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("unsupported format version: {0}")]
    UnsupportedVersion(String),

    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: io::Error,
    },
}

impl Error {
    /// Stable identifier for the error kind.
    pub fn code(&self) -> &'static str {
        match self {
            Error::SourceUnavailable(_) => "SourceUnavailable",
            Error::PermissionLost(_) => "PermissionLost",
            Error::SourceChanging => "SourceChanging",
            Error::BundleIncomplete(_) => "BundleIncomplete",
            Error::StorageFull { side: StorageSide::Vault, .. } => "VaultFull",
            Error::StorageFull { side: StorageSide::SaveLocation, .. } => "SaveLocationFull",
            Error::IntegrityFailed(_) => "IntegrityFailed",
            Error::IncompatibleSave(_) => "IncompatibleSave",
            Error::LocalChangedSinceReview { .. } => "LocalChangedSinceReview",
            Error::BackupUnverified(_) => "BackupUnverified",
            Error::StrategyUnsupported(_) => "StrategyUnsupported",
            Error::InstallInterrupted(_) => "InstallInterrupted",
            Error::RecoveryRequired => "RecoveryRequired",
            Error::RecoveryFailed(_) => "RecoveryFailed",
            Error::Busy => "Busy",
            Error::NotFound(_) => "NotFound",
            Error::InvalidInput(_) => "InvalidInput",
            Error::UnsupportedVersion(_) => "UnsupportedVersion",
            Error::Io { .. } => "Io",
        }
    }

    /// Whether playable files might differ from both the previous save and
    /// the intended one. Only interrupted installs can leave that state.
    pub fn playable_files_may_be_mixed(&self) -> bool {
        matches!(
            self,
            Error::InstallInterrupted(_) | Error::RecoveryRequired | Error::RecoveryFailed(_)
        )
    }

    /// Wrap an I/O error, classifying conditions the user can act on.
    pub fn io(context: impl Into<String>, side: StorageSide, source: io::Error) -> Self {
        let context = context.into();
        match source.kind() {
            io::ErrorKind::StorageFull | io::ErrorKind::QuotaExceeded => {
                Error::StorageFull { side, detail: format!("{context}: {source}") }
            }
            io::ErrorKind::PermissionDenied if side == StorageSide::SaveLocation => {
                Error::PermissionLost(format!("{context}: {source}"))
            }
            _ => Error::Io { context, source },
        }
    }
}
