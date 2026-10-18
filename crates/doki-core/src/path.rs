//! Portable paths inside a save bundle.

use std::fmt;

use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

use crate::error::{Error, Result};

/// Maximum length of one bundle path in bytes, after normalization.
pub const MAX_PATH_BYTES: usize = 512;

/// Names Windows refuses as file names regardless of extension.
const WINDOWS_RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Directory name Doki reserves inside save folders for staging installs.
pub const RESERVED_DIR: &str = ".doki-staging";

/// A relative path of a file inside a bundle root.
///
/// Normalized to Unicode NFC with `/` separators so the same file has the same
/// path, and the same checkpoint the same id, on Windows and Android. Paths
/// that could escape the bundle root or that a supported platform cannot
/// store are rejected at capture and again when a checkpoint is received.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct BundlePath(String);

impl BundlePath {
    /// Validate and normalize. Accepts `\` as a separator on input.
    pub fn new(raw: &str) -> Result<Self> {
        let normalized: String = raw.replace('\\', "/").nfc().collect();
        let bad = |why: &str| Err(Error::InvalidInput(format!("bundle path {raw:?}: {why}")));
        if normalized.is_empty() {
            return bad("empty");
        }
        if normalized.len() > MAX_PATH_BYTES {
            return bad("too long");
        }
        if normalized.starts_with('/') {
            return bad("absolute");
        }
        for segment in normalized.split('/') {
            if segment.is_empty() || segment == "." || segment == ".." {
                return bad("empty, '.' or '..' segment");
            }
            if segment == RESERVED_DIR {
                return bad("reserved by Doki");
            }
            if segment.chars().any(|c| c.is_control() || "<>:\"|?*".contains(c)) {
                return bad("character not allowed on Windows");
            }
            if segment.ends_with('.') || segment.ends_with(' ') {
                return bad("trailing dot or space");
            }
            let stem = segment.split('.').next().unwrap_or("").to_ascii_uppercase();
            if WINDOWS_RESERVED.contains(&stem.as_str()) {
                return bad("reserved Windows device name");
            }
        }
        Ok(BundlePath(normalized))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn segments(&self) -> impl Iterator<Item = &str> {
        self.0.split('/')
    }

    /// Key used to detect paths that would collide on a case-insensitive
    /// filesystem (Windows, most Android shared storage).
    pub fn collision_key(&self) -> String {
        self.0.to_lowercase()
    }
}

impl TryFrom<String> for BundlePath {
    type Error = Error;
    fn try_from(s: String) -> Result<Self> {
        Self::new(&s)
    }
}

impl From<BundlePath> for String {
    fn from(p: BundlePath) -> String {
        p.0
    }
}

impl fmt::Display for BundlePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for BundlePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BundlePath({:?})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_separators_and_unicode() {
        let decomposed = "saves\\Pok\u{0065}\u{0301}mon.sav";
        let p = BundlePath::new(decomposed).unwrap();
        assert_eq!(p.as_str(), "saves/Pok\u{00e9}mon.sav");
    }

    #[test]
    fn rejects_escapes_and_unportable_names() {
        for bad in [
            "",
            "/abs",
            "a/../b",
            "./a",
            "a//b",
            "con.sav",
            "a/NUL",
            "x:y",
            "trail.",
            "sp ",
            ".doki-staging/x",
        ] {
            assert!(BundlePath::new(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn collision_key_is_case_insensitive() {
        let a = BundlePath::new("Game.SAV").unwrap();
        let b = BundlePath::new("game.sav").unwrap();
        assert_ne!(a, b);
        assert_eq!(a.collision_key(), b.collision_key());
    }
}
