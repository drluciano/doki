//! Relay client: pairing, end-to-end encryption and sync.
//!
//! # Pairing
//!
//! The first device creates a [`RelayLink`]: the relay URL, a random space id
//! and a random 32-byte secret. Other devices join by entering its pairing
//! code (shown as a QR code by the app). Every device holding the code can
//! read and publish progress in that space.
//!
//! From the secret, HKDF-SHA256 derives two independent keys:
//! - an access token sent to the relay (the relay stores only its hash);
//! - a content key that never leaves the devices.
//!
//! # Encryption
//!
//! Each handoff package is sealed with XChaCha20-Poly1305 under the content
//! key, a random 24-byte nonce and the space id as associated data:
//!
//! ```text
//! b"DOKIENC1" | nonce (24 bytes) | ciphertext+tag
//! ```
//!
//! The relay sees only blob sizes and upload times. A blob that fails to
//! decrypt is reported and never imported.
//!
//! **Interim design.** All paired devices share one secret, so a lost device
//! can only be shut out by creating a new space and re-pairing the others.
//! Per-device keys, revocation and key recovery are designed in S01 (#25)
//! and required before internet-facing release.
//!
//! # Sync
//!
//! [`sync`] uploads this device's current save of each game when it changed
//! since the last upload, then downloads and imports every newer blob from
//! other devices. Importing only ever writes the vault: incoming progress
//! still waits for review.

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::time::Duration;

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use doki_core::{BundleId, CheckpointId, Error, Result};
use doki_engine::{Engine, ImportSummary, LocationProvider};
use doki_vault::random_hex;
use hkdf::Hkdf;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

const BLOB_MAGIC: &[u8; 8] = b"DOKIENC1";
const CODE_PREFIX: &str = "DOKI1:";
const RECORD_KIND: &str = "relay";
const MAX_BLOB_BYTES: u64 = 256 * 1024 * 1024;

/// Connection to one relay space. Stored in the vault.
#[derive(Clone, Serialize, Deserialize)]
pub struct RelayLink {
    pub url: String,
    pub space: String,
    /// 32-byte secret, hex. Grants full access to the space.
    secret: String,
}

impl std::fmt::Debug for RelayLink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RelayLink").field("url", &self.url).field("space", &self.space).finish()
    }
}

impl RelayLink {
    /// A new space on the relay at `url`.
    pub fn create(url: &str) -> Result<Self> {
        let url = normalize_url(url)?;
        Ok(RelayLink { url, space: random_hex(16), secret: random_hex(32) })
    }

    /// Pairing code for other devices: `DOKI1:<url>#<space>.<secret>`.
    /// Anyone with the code can read your progress; share it only with your
    /// own devices.
    pub fn pairing_code(&self) -> String {
        format!("{CODE_PREFIX}{}#{}.{}", self.url, self.space, self.secret)
    }

    pub fn from_pairing_code(code: &str) -> Result<Self> {
        let bad = || Error::InvalidInput("that is not a Doki pairing code".into());
        let rest = code.trim().strip_prefix(CODE_PREFIX).ok_or_else(bad)?;
        let (url, ids) = rest.rsplit_once('#').ok_or_else(bad)?;
        let (space, secret) = ids.split_once('.').ok_or_else(bad)?;
        let hex_ok = |s: &str, n: usize| {
            s.len() == n && s.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        };
        if !hex_ok(space, 32) || !hex_ok(secret, 64) {
            return Err(bad());
        }
        Ok(RelayLink { url: normalize_url(url)?, space: space.into(), secret: secret.into() })
    }

    fn derive(&self, info: &[u8]) -> [u8; 32] {
        let secret = hex::decode(&self.secret).expect("validated hex");
        let mut out = [0u8; 32];
        Hkdf::<Sha256>::new(Some(self.space.as_bytes()), &secret)
            .expand(info, &mut out)
            .expect("32 bytes is a valid HKDF length");
        out
    }

    fn token(&self) -> String {
        hex::encode(self.derive(b"doki relay access token v1"))
    }

    fn cipher(&self) -> XChaCha20Poly1305 {
        XChaCha20Poly1305::new(&self.derive(b"doki package content key v1").into())
    }

    /// Encrypt a package for upload.
    pub fn seal(&self, plaintext: &[u8]) -> Vec<u8> {
        let mut nonce = [0u8; 24];
        getrandom::fill(&mut nonce).expect("system random source available");
        let ct = self
            .cipher()
            .encrypt(
                XNonce::from_slice(&nonce),
                Payload { msg: plaintext, aad: self.space.as_bytes() },
            )
            .expect("encryption of in-memory data cannot fail");
        [BLOB_MAGIC.as_slice(), &nonce, &ct].concat()
    }

    /// Decrypt and authenticate a downloaded blob.
    pub fn open(&self, blob: &[u8]) -> Result<Vec<u8>> {
        let bad = || {
            Error::IntegrityFailed("a relay package could not be decrypted or was altered".into())
        };
        if blob.len() < 8 + 24 + 16 || &blob[..8] != BLOB_MAGIC {
            return Err(bad());
        }
        self.cipher()
            .decrypt(
                XNonce::from_slice(&blob[8..32]),
                Payload { msg: &blob[32..], aad: self.space.as_bytes() },
            )
            .map_err(|_| bad())
    }
}

fn normalize_url(url: &str) -> Result<String> {
    let url = url.trim().trim_end_matches('/');
    if !(url.starts_with("http://") || url.starts_with("https://")) || url.contains('#') {
        return Err(Error::InvalidInput(
            "relay address must start with http:// or https://".into(),
        ));
    }
    Ok(url.to_owned())
}

/// Sync progress kept in the vault.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct SyncState {
    /// Highest relay sequence number handled.
    last_seq: u64,
    /// Head last uploaded for each game.
    sent: BTreeMap<BundleId, CheckpointId>,
}

/// What one sync did.
#[derive(Debug, Default)]
pub struct SyncReport {
    /// Games whose current save was uploaded.
    pub uploaded: Vec<BundleId>,
    /// One entry per imported package from another device.
    pub received: Vec<ImportSummary>,
    /// Packages that could not be used (e.g. failed to decrypt). They are
    /// skipped, not retried.
    pub rejected: Vec<(u64, String)>,
}

pub fn save_link<P: LocationProvider>(engine: &Engine<P>, link: &RelayLink) -> Result<()> {
    engine.vault().put_record(RECORD_KIND, "link", link)?;
    engine.vault().put_record(RECORD_KIND, "state", &SyncState::default())
}

pub fn load_link<P: LocationProvider>(engine: &Engine<P>) -> Result<Option<RelayLink>> {
    engine.vault().get_record(RECORD_KIND, "link")
}

pub fn forget_link<P: LocationProvider>(engine: &Engine<P>) -> Result<()> {
    engine.vault().delete_record(RECORD_KIND, "link")?;
    engine.vault().delete_record(RECORD_KIND, "state")
}

struct Client<'a> {
    link: &'a RelayLink,
    agent: ureq::Agent,
}

fn net_err(what: &str, e: ureq::Error) -> Error {
    match e {
        ureq::Error::Status(401, _) => {
            Error::PermissionLost(format!("the relay refused this device's pairing ({what})"))
        }
        ureq::Error::Status(413, _) => {
            Error::InvalidInput("the save is larger than the relay accepts".into())
        }
        ureq::Error::Status(507, _) => Error::StorageFull {
            side: doki_core::error::StorageSide::Vault,
            detail: "the relay's storage for your devices is full".into(),
        },
        ureq::Error::Status(code, _) => {
            Error::SourceUnavailable(format!("relay answered {code} ({what})"))
        }
        ureq::Error::Transport(t) => {
            Error::SourceUnavailable(format!("relay unreachable ({what}): {t}"))
        }
    }
}

impl<'a> Client<'a> {
    fn new(link: &'a RelayLink) -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(15))
            .timeout(Duration::from_secs(300))
            .build();
        Client { link, agent }
    }

    fn space_url(&self) -> String {
        format!("{}/v1/spaces/{}", self.link.url, self.link.space)
    }

    fn auth(&self) -> String {
        format!("Bearer {}", self.link.token())
    }

    fn register(&self) -> Result<()> {
        self.agent
            .put(&self.space_url())
            .set("Authorization", &self.auth())
            .call()
            .map_err(|e| net_err("connecting", e))?;
        Ok(())
    }

    fn upload(&self, device: &str, blob: &[u8]) -> Result<u64> {
        #[derive(Deserialize)]
        struct Seq {
            seq: u64,
        }
        let r = self
            .agent
            .post(&format!("{}/packages", self.space_url()))
            .set("Authorization", &self.auth())
            .set("X-Doki-Device", device)
            .send_bytes(blob)
            .map_err(|e| net_err("uploading", e))?;
        let seq: Seq = serde_json::from_reader(r.into_reader())
            .map_err(|e| Error::SourceUnavailable(format!("relay reply: {e}")))?;
        Ok(seq.seq)
    }

    fn list(&self, after: u64) -> Result<Vec<RemotePackage>> {
        let r = self
            .agent
            .get(&format!("{}/packages?after={after}", self.space_url()))
            .set("Authorization", &self.auth())
            .call()
            .map_err(|e| net_err("listing", e))?;
        serde_json::from_reader(r.into_reader())
            .map_err(|e| Error::SourceUnavailable(format!("relay reply: {e}")))
    }

    fn download(&self, seq: u64) -> Result<Vec<u8>> {
        let r = self
            .agent
            .get(&format!("{}/packages/{seq}", self.space_url()))
            .set("Authorization", &self.auth())
            .call()
            .map_err(|e| net_err("downloading", e))?;
        let mut body = Vec::new();
        r.into_reader()
            .take(MAX_BLOB_BYTES + 1)
            .read_to_end(&mut body)
            .map_err(|e| Error::SourceUnavailable(format!("download interrupted: {e}")))?;
        if body.len() as u64 > MAX_BLOB_BYTES {
            return Err(Error::InvalidInput("relay package is too large".into()));
        }
        Ok(body)
    }
}

#[derive(Debug, Deserialize)]
struct RemotePackage {
    seq: u64,
    from_device: String,
}

/// Check that the relay accepts this link (creating the space on first use).
pub fn connect(link: &RelayLink) -> Result<()> {
    Client::new(link).register()
}

/// Upload changed saves, then download and import progress from other
/// devices. Never changes save files.
pub fn sync<P: LocationProvider>(engine: &Engine<P>) -> Result<SyncReport> {
    let link = load_link(engine)?
        .ok_or_else(|| Error::NotFound("this device is not paired with a relay".into()))?;
    let client = Client::new(&link);
    client.register()?;
    let vault = engine.vault();
    let mut state: SyncState = vault.get_record(RECORD_KIND, "state")?.unwrap_or_default();
    let mut report = SyncReport::default();
    let device = vault.device_id().to_string();

    // Upload first, so a device that both sends and receives publishes its
    // own progress before learning of others'. Order does not affect safety.
    let changed: Vec<BundleId> = engine
        .bundles()?
        .into_iter()
        .filter(|b| b.head.is_some() && state.sent.get(&b.bundle_id) != b.head.as_ref())
        .map(|b| b.bundle_id)
        .collect();
    if !changed.is_empty() {
        let tmp = vault.root().join("tmp").join(format!("{}.dokipkg", random_hex(8)));
        let result = engine.export_package(&changed, &tmp).and_then(|_| {
            fs::read(&tmp)
                .map_err(|e| Error::io("reading package", doki_core::error::StorageSide::Vault, e))
        });
        let _ = fs::remove_file(&tmp);
        client.upload(&device, &link.seal(&result?))?;
        for b in &changed {
            if let Some(head) = engine.bundle(b)?.head {
                state.sent.insert(b.clone(), head);
            }
        }
        vault.put_record(RECORD_KIND, "state", &state)?;
        report.uploaded = changed;
    }

    for pkg in client.list(state.last_seq)? {
        if pkg.from_device != device {
            let outcome =
                client.download(pkg.seq).and_then(|blob| link.open(&blob)).and_then(|plain| {
                    let tmp = vault.root().join("tmp").join(format!("{}.dokipkg", random_hex(8)));
                    let written = fs::write(&tmp, plain).map_err(|e| {
                        Error::io("writing package", doki_core::error::StorageSide::Vault, e)
                    });
                    let imported = written.and_then(|_| engine.import_package(&tmp));
                    let _ = fs::remove_file(&tmp);
                    imported
                });
            match outcome {
                Ok(summary) => report.received.push(summary),
                // Network trouble: stop here and retry from this package next time.
                Err(e @ (Error::SourceUnavailable(_) | Error::PermissionLost(_) | Error::Busy)) => {
                    return Err(e)
                }
                Err(e) => report.rejected.push((pkg.seq, e.to_string())),
            }
        }
        state.last_seq = pkg.seq;
        vault.put_record(RECORD_KIND, "state", &state)?;
    }
    Ok(report)
}

#[cfg(test)]
mod tests;
