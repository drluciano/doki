//! Content hashes.

use std::fmt;
use std::io::{self, Read};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{Error, Result};

const PREFIX: &str = "sha256:";

/// SHA-256 of a byte sequence, written as `sha256:<64 lowercase hex>`.
///
/// A matching hash proves bytes are unchanged. It says nothing about whether
/// an emulator can load them (safety contract I9).
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ContentHash(String);

impl ContentHash {
    pub fn of_bytes(bytes: &[u8]) -> Self {
        Self::from_digest(Sha256::digest(bytes).into())
    }

    /// Hash everything `reader` yields, returning the hash and byte count.
    pub fn of_reader(reader: &mut impl Read) -> io::Result<(Self, u64)> {
        let mut hasher = Hasher::new();
        let mut buf = [0u8; 64 * 1024];
        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        Ok(hasher.finish())
    }

    fn from_digest(digest: [u8; 32]) -> Self {
        ContentHash(format!("{PREFIX}{}", hex::encode(digest)))
    }

    pub fn parse(s: &str) -> Result<Self> {
        let hex_part = s
            .strip_prefix(PREFIX)
            .ok_or_else(|| Error::InvalidInput(format!("hash must start with {PREFIX}: {s}")))?;
        if hex_part.len() != 64
            || !hex_part.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(Error::InvalidInput(format!("malformed hash: {s}")));
        }
        Ok(ContentHash(s.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The 64-character hex part, safe to use as a file name.
    pub fn hex(&self) -> &str {
        &self.0[PREFIX.len()..]
    }
}

impl TryFrom<String> for ContentHash {
    type Error = Error;
    fn try_from(s: String) -> Result<Self> {
        Self::parse(&s)
    }
}

impl From<ContentHash> for String {
    fn from(h: ContentHash) -> String {
        h.0
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ContentHash({})", &self.0[..PREFIX.len() + 12])
    }
}

/// Incremental hasher that also counts bytes.
pub struct Hasher {
    inner: Sha256,
    len: u64,
}

impl Hasher {
    pub fn new() -> Self {
        Hasher { inner: Sha256::new(), len: 0 }
    }

    pub fn update(&mut self, bytes: &[u8]) {
        self.inner.update(bytes);
        self.len += bytes.len() as u64;
    }

    /// Bytes hashed so far.
    pub fn len(&self) -> u64 {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn finish(self) -> (ContentHash, u64) {
        (ContentHash::from_digest(self.inner.finalize().into()), self.len)
    }
}

impl Default for Hasher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vector() {
        assert_eq!(
            ContentHash::of_bytes(b"abc").as_str(),
            "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn rejects_malformed() {
        assert!(ContentHash::parse("md5:abc").is_err());
        assert!(ContentHash::parse("sha256:ABC").is_err());
        assert!(ContentHash::parse(&format!("sha256:{}", "g".repeat(64))).is_err());
    }

    #[test]
    fn reader_matches_bytes() {
        let data = vec![7u8; 200_000];
        let (h, n) = ContentHash::of_reader(&mut &data[..]).unwrap();
        assert_eq!(n, 200_000);
        assert_eq!(h, ContentHash::of_bytes(&data));
    }
}
