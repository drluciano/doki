//! Doki domain model.
//!
//! This crate is pure: it never touches the filesystem or network. It defines
//! what a checkpoint is, how its identity is computed, how bundle paths are
//! validated, how two checkpoints relate through ancestry, and the stable
//! error taxonomy every other crate reports through.
//!
//! The safety contract these types serve is `docs/spec/safety-contract.md`.

pub mod ancestry;
pub mod error;
pub mod hash;
pub mod model;
pub mod path;

pub use ancestry::{relate, Graph, Relation};
pub use error::{Error, Result};
pub use hash::ContentHash;
pub use model::{
    BundleId, CaptureMode, Checkpoint, CheckpointId, CheckpointKind, DeviceId, FileEntry,
    FilesDigest, ProfileRef, CHECKPOINT_SCHEMA_VERSION,
};
pub use path::BundlePath;
