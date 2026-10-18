//! Checkpoints and their identity.

use std::collections::HashSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::hash::ContentHash;
use crate::path::BundlePath;

/// Current checkpoint schema. Readers refuse checkpoints with a newer major
/// version instead of guessing at their meaning.
pub const CHECKPOINT_SCHEMA_VERSION: u32 = 1;

/// Maximum files in one bundle. Emulator saves are small sets; a larger
/// number indicates a misconfigured folder or a hostile manifest.
pub const MAX_FILES_PER_BUNDLE: usize = 4096;

macro_rules! id_type {
    ($(#[$m:meta])* $name:ident) => {
        $(#[$m])*
        #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            /// Accepts 8 to 64 lowercase hex characters.
            pub fn parse(s: &str) -> Result<Self> {
                if (8..=64).contains(&s.len())
                    && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                {
                    Ok($name(s.to_owned()))
                } else {
                    Err(Error::InvalidInput(format!(
                        concat!("malformed ", stringify!($name), ": {:?}"),
                        s
                    )))
                }
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Short form for display.
            pub fn short(&self) -> &str {
                &self.0[..8]
            }
        }

        impl TryFrom<String> for $name {
            type Error = Error;
            fn try_from(s: String) -> Result<Self> {
                Self::parse(&s)
            }
        }

        impl From<$name> for String {
            fn from(v: $name) -> String {
                v.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!(stringify!($name), "({})"), self.short())
            }
        }
    };
}

id_type!(
    /// Hash of a checkpoint's canonical form; see [`Checkpoint::id`].
    CheckpointId
);
id_type!(
    /// Stable identity of one game's save across devices, independent of
    /// folder names and paths. Shared between devices by explicit mapping.
    BundleId
);
id_type!(
    /// Identity of one installation of Doki.
    DeviceId
);

/// Why a checkpoint exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CheckpointKind {
    /// The user (or a supported integration) captured the playable save.
    Capture,
    /// Captured automatically just before playable files were replaced.
    Backup,
    /// Records a decision between continuations. Its files equal the first
    /// parent's; the other parents are set aside but stay in history.
    Choice,
}

/// How confident the capture was that the emulator was not writing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CaptureMode {
    /// The user confirmed the game was closed. MVP default.
    EmulatorStoppedConfirmed,
    /// A tested emulator integration reported a safe session boundary.
    IntegrationAttested,
    /// Only file-stability heuristics were available.
    QuiescenceHeuristic,
    /// Taken by Doki itself immediately before an install (backups) or for a
    /// choice record, after a re-read confirmed the files were stable.
    Automatic,
}

/// The emulator profile that defined which files form the bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileRef {
    pub id: String,
    pub version: u32,
}

/// One file of a bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: BundlePath,
    pub size: u64,
    pub hash: ContentHash,
}

/// Hash of a sorted file list. Equal digests mean byte-identical bundles,
/// regardless of which checkpoint or device produced them.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FilesDigest(pub ContentHash);

impl FilesDigest {
    pub fn of(files: &[FileEntry]) -> Self {
        let canonical = serde_json::to_vec(files).expect("file entries serialize");
        FilesDigest(ContentHash::of_bytes(&canonical))
    }
}

/// An immutable record of one bundle's exact bytes and where it came from.
///
/// Field order is part of the identity: `id()` hashes the JSON
/// serialization, which follows declaration order. Do not reorder fields;
/// add new optional fields at the end and bump the schema version when
/// meaning changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoint {
    pub schema_version: u32,
    pub bundle_id: BundleId,
    /// Parent checkpoints. Empty only for the first capture of a bundle on a
    /// device. For [`CheckpointKind::Choice`] the first parent is the kept
    /// continuation.
    pub parents: Vec<CheckpointId>,
    pub kind: CheckpointKind,
    /// Sorted by path; see [`Checkpoint::validate`].
    pub files: Vec<FileEntry>,
    pub profile: ProfileRef,
    pub origin_device: DeviceId,
    /// The device's name when the checkpoint was made, for display.
    pub origin_device_name: String,
    /// Device clock in RFC 3339. Descriptive only: never used to decide
    /// which progress wins (I4).
    pub captured_at: String,
    pub capture_mode: CaptureMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl Checkpoint {
    /// Content identity: SHA-256 of the canonical JSON form.
    pub fn id(&self) -> CheckpointId {
        let canonical = serde_json::to_vec(self).expect("checkpoint serializes");
        CheckpointId(ContentHash::of_bytes(&canonical).hex().to_owned())
    }

    pub fn files_digest(&self) -> FilesDigest {
        FilesDigest::of(&self.files)
    }

    pub fn total_size(&self) -> u64 {
        self.files.iter().map(|f| f.size).sum()
    }

    /// Structural checks applied at capture and again when a checkpoint is
    /// received from another device, which may be malicious or buggy.
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != CHECKPOINT_SCHEMA_VERSION {
            return Err(Error::UnsupportedVersion(format!(
                "checkpoint schema {} (this build reads {})",
                self.schema_version, CHECKPOINT_SCHEMA_VERSION
            )));
        }
        if self.files.len() > MAX_FILES_PER_BUNDLE {
            return Err(Error::InvalidInput(format!("{} files in one bundle", self.files.len())));
        }
        if self.files.is_empty() {
            return Err(Error::BundleIncomplete("checkpoint has no files".into()));
        }
        if !self.files.windows(2).all(|w| w[0].path < w[1].path) {
            return Err(Error::InvalidInput("files are not strictly sorted by path".into()));
        }
        let mut keys = HashSet::new();
        for f in &self.files {
            if !keys.insert(f.path.collision_key()) {
                return Err(Error::InvalidInput(format!(
                    "{} collides with another file on case-insensitive storage",
                    f.path
                )));
            }
        }
        // A file and a directory with the same name cannot coexist.
        for f in &self.files {
            let prefix = format!("{}/", f.path.collision_key());
            if keys.iter().any(|k| k.starts_with(&prefix)) {
                return Err(Error::InvalidInput(format!("{} is both a file and a folder", f.path)));
            }
        }
        let unique: HashSet<_> = self.parents.iter().collect();
        if unique.len() != self.parents.len() {
            return Err(Error::InvalidInput("duplicate parent".into()));
        }
        match self.kind {
            CheckpointKind::Choice if self.parents.len() < 2 => {
                Err(Error::InvalidInput("a choice needs at least two parents".into()))
            }
            CheckpointKind::Capture | CheckpointKind::Backup if self.parents.len() > 1 => {
                Err(Error::InvalidInput("a capture has at most one parent".into()))
            }
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn sample(parents: Vec<CheckpointId>, content: &[u8]) -> Checkpoint {
        Checkpoint {
            schema_version: CHECKPOINT_SCHEMA_VERSION,
            bundle_id: BundleId::parse("00112233aabbccdd").unwrap(),
            parents,
            kind: CheckpointKind::Capture,
            files: vec![FileEntry {
                path: BundlePath::new("game.sav").unwrap(),
                size: content.len() as u64,
                hash: ContentHash::of_bytes(content),
            }],
            profile: ProfileRef { id: "generic-folder".into(), version: 1 },
            origin_device: DeviceId::parse("deadbeefdeadbeef").unwrap(),
            origin_device_name: "PC".into(),
            captured_at: "2026-10-18T20:42:00Z".into(),
            capture_mode: CaptureMode::EmulatorStoppedConfirmed,
            note: None,
        }
    }

    #[test]
    fn id_is_stable_and_sensitive() {
        let a = sample(vec![], b"one");
        assert_eq!(a.id(), a.clone().id());
        assert_ne!(a.id(), sample(vec![], b"two").id());
        let mut later = a.clone();
        later.captured_at = "2026-10-19T00:00:00Z".into();
        assert_ne!(a.id(), later.id(), "metadata is part of identity");
        assert_eq!(a.files_digest(), later.files_digest(), "but not of content");
    }

    #[test]
    fn id_survives_json_round_trip() {
        let a = sample(vec![], b"one");
        let back: Checkpoint = serde_json::from_slice(&serde_json::to_vec(&a).unwrap()).unwrap();
        assert_eq!(a.id(), back.id());
    }

    #[test]
    fn validate_rejects_bad_shapes() {
        let ok = sample(vec![], b"x");
        ok.validate().unwrap();

        let mut empty = ok.clone();
        empty.files.clear();
        assert_eq!(empty.validate().unwrap_err().code(), "BundleIncomplete");

        let mut collide = ok.clone();
        let mut upper = collide.files[0].clone();
        upper.path = BundlePath::new("GAME.SAV").unwrap();
        collide.files.insert(0, upper);
        assert!(collide.validate().is_err());

        let mut file_and_dir = ok.clone();
        let mut nested = file_and_dir.files[0].clone();
        nested.path = BundlePath::new("game.sav/inner").unwrap();
        file_and_dir.files.push(nested);
        assert!(file_and_dir.validate().is_err());

        let mut choice = ok.clone();
        choice.kind = CheckpointKind::Choice;
        assert!(choice.validate().is_err());

        let mut future = ok;
        future.schema_version = 99;
        assert_eq!(future.validate().unwrap_err().code(), "UnsupportedVersion");
    }
}
