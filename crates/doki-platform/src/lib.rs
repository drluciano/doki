//! Save-location adapters.
//!
//! A [`SaveLocation`] is the emulator's real save folder for one bundle. It is
//! the only interface through which Doki changes playable files, and the
//! engine only calls its mutating methods from the journaled install
//! procedure. Adapters report what they can do instead of claiming universal
//! atomicity: a multi-file bundle is never replaced atomically, and per-file
//! atomic replacement must be evidenced per platform (see
//! `docs/spec/safety-contract.md` section 9).
//!
//! This crate provides [`local::LocalDir`] for ordinary desktop folders and
//! [`faults::FaultyLocation`] for fault-injection tests. The Android
//! Storage Access Framework adapter lives behind the same trait.

use std::io::Read;

use doki_core::{BundlePath, Result};

pub mod faults;
pub mod local;

/// Whether an adapter can perform an operation with a given guarantee.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Support {
    /// Measured or documented for this platform and storage type.
    Supported,
    Unsupported,
    /// Not established. The engine treats this as unsupported.
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capabilities {
    /// Replacing one file either leaves the old bytes or the new bytes,
    /// never a truncated mix. Required for installs in this release.
    pub per_file_atomic_replace: Support,
    /// Written data can be flushed to stable storage before the call returns.
    pub durable_flush: Support,
}

/// One emulator save folder.
///
/// Paths are always relative to the folder root and validated by
/// [`BundlePath`]. Staged files live in an adapter-private area associated
/// with an operation id so a crash leaves them identifiable and removable.
pub trait SaveLocation {
    /// Human-readable location for local display. May contain a path.
    fn describe(&self) -> String;

    fn capabilities(&self) -> Capabilities;

    /// Fails with `SourceUnavailable` or `PermissionLost` when the folder
    /// cannot be reached. A missing folder is never reported as empty.
    fn check_available(&self) -> Result<()>;

    /// All regular files that form the bundle, sorted. Excludes Doki's
    /// staging area.
    fn list(&self) -> Result<Vec<BundlePath>>;

    fn open(&self, path: &BundlePath) -> Result<Box<dyn Read + '_>>;

    /// Write `data` into the staging area for `op` and flush it. Does not
    /// touch playable files.
    fn stage(&self, op: &str, path: &BundlePath, data: &mut dyn Read) -> Result<()>;

    fn open_staged(&self, op: &str, path: &BundlePath) -> Result<Box<dyn Read + '_>>;

    /// Replace (or create) the playable file at `path` with the staged copy.
    /// Destructive: callers must hold a verified backup and an open journal.
    fn commit_staged(&self, op: &str, path: &BundlePath) -> Result<()>;

    /// Remove a playable file. Destructive, same preconditions as
    /// [`SaveLocation::commit_staged`]. Removing an absent file succeeds.
    fn remove(&self, path: &BundlePath) -> Result<()>;

    /// Delete everything staged for `op`. Never touches playable files.
    fn discard_staging(&self, op: &str) -> Result<()>;
}
