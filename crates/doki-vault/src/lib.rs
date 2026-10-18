//! The local vault.
//!
//! Layout under the vault root (format 1):
//!
//! ```text
//! vault.json                 device identity and format version
//! objects/ab/<hex>           immutable file contents, named by SHA-256
//! checkpoints/<id>.json      immutable checkpoints, named by their id
//! records/<kind>/<key>.json  mutable records (bundles, proposals, journals)
//! locks/                     OS advisory locks; released when a process dies
//! tmp/                       in-progress writes; safe to delete when unlocked
//! ```
//!
//! Every write goes to `tmp/`, is flushed, then renamed into place, so a
//! crash leaves either the old file, the new file, or an orphan in `tmp/`.
//! Objects and checkpoints are written once and never modified (I1). A
//! checkpoint is only written after all its objects are present (I8).
//!
//! JSON files are the source of truth. A faster index can be added later but
//! must be rebuildable from these files.

use std::collections::HashSet;
use std::fs::{self, File, TryLockError};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use doki_core::error::StorageSide::Vault as Side;
use doki_core::hash::Hasher;
use doki_core::{Checkpoint, CheckpointId, ContentHash, DeviceId, Error, FileEntry, Graph, Result};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub const VAULT_FORMAT: u32 = 1;

/// Upper bound for a single record or checkpoint file, to bound memory when
/// reading data that may have come from another device.
const MAX_RECORD_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    pub format: u32,
    pub device_id: DeviceId,
    pub device_name: String,
}

pub struct Vault {
    root: PathBuf,
    config: VaultConfig,
}

/// Random lowercase hex identifier of `bytes` random bytes.
pub fn random_hex(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    getrandom::fill(&mut buf).expect("system random source available");
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

/// Current time in RFC 3339 (UTC, seconds). Descriptive only.
pub fn now_rfc3339() -> String {
    humantime::format_rfc3339_seconds(SystemTime::now()).to_string()
}

/// Parse a time written by [`now_rfc3339`]. Times only ever decide
/// retention, never which save wins.
pub fn parse_rfc3339(s: &str) -> Option<SystemTime> {
    humantime::parse_rfc3339_weak(s).ok()
}

fn io_err(context: impl Into<String>) -> impl FnOnce(io::Error) -> Error {
    let context = context.into();
    move |e| Error::io(context, Side, e)
}

fn sync_dir(dir: &Path) {
    #[cfg(unix)]
    if let Ok(d) = File::open(dir) {
        let _ = d.sync_all();
    }
    #[cfg(not(unix))]
    let _ = dir;
}

impl Vault {
    /// Create a new vault. Fails if one already exists at `root`.
    pub fn init(root: impl Into<PathBuf>, device_name: &str) -> Result<Self> {
        let root = root.into();
        if root.join("vault.json").exists() {
            return Err(Error::InvalidInput(format!(
                "a vault already exists at {}",
                root.display()
            )));
        }
        for dir in ["objects", "checkpoints", "records", "locks", "tmp"] {
            fs::create_dir_all(root.join(dir)).map_err(io_err("creating vault"))?;
        }
        let config = VaultConfig {
            format: VAULT_FORMAT,
            device_id: DeviceId::parse(&random_hex(8))?,
            device_name: device_name.to_owned(),
        };
        let vault = Vault { root, config };
        vault.write_atomic(
            &vault.root.join("vault.json"),
            &serde_json::to_vec_pretty(&vault.config).unwrap(),
        )?;
        Ok(vault)
    }

    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        let path = root.join("vault.json");
        if !path.exists() {
            return Err(Error::NotFound(format!("no vault at {}", root.display())));
        }
        let config: VaultConfig = read_json(&path)?;
        if config.format != VAULT_FORMAT {
            return Err(Error::UnsupportedVersion(format!("vault format {}", config.format)));
        }
        Ok(Vault { root, config })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn device_id(&self) -> &DeviceId {
        &self.config.device_id
    }

    pub fn device_name(&self) -> &str {
        &self.config.device_name
    }

    // ---- atomic files -------------------------------------------------

    fn tmp_path(&self) -> PathBuf {
        self.root.join("tmp").join(random_hex(12))
    }

    /// Write `bytes` to `dest` so readers see either the old or new content.
    fn write_atomic(&self, dest: &Path, bytes: &[u8]) -> Result<()> {
        let tmp = self.tmp_path();
        let result = (|| {
            let mut f = File::create(&tmp)?;
            f.write_all(bytes)?;
            f.sync_all()?;
            let parent = dest.parent().expect("vault paths have parents");
            fs::create_dir_all(parent)?;
            fs::rename(&tmp, dest)?;
            sync_dir(parent);
            Ok(())
        })();
        result.map_err(|e| {
            let _ = fs::remove_file(&tmp);
            Error::io(format!("writing {}", dest.display()), Side, e)
        })
    }

    // ---- objects ------------------------------------------------------

    pub fn object_path(&self, hash: &ContentHash) -> PathBuf {
        let hex = hash.hex();
        self.root.join("objects").join(&hex[..2]).join(hex)
    }

    pub fn has_object(&self, hash: &ContentHash) -> bool {
        self.object_path(hash).is_file()
    }

    /// Store everything `data` yields as an object. Idempotent: identical
    /// content is stored once. `expect` rejects data whose hash or size
    /// differs (used when receiving from another device).
    pub fn put_object(
        &self,
        data: &mut dyn Read,
        expect: Option<(&ContentHash, u64)>,
    ) -> Result<(ContentHash, u64)> {
        let tmp = self.tmp_path();
        let result = (|| -> Result<(ContentHash, u64)> {
            let mut f = File::create(&tmp).map_err(io_err("creating object"))?;
            let mut hasher = Hasher::new();
            let mut buf = [0u8; 64 * 1024];
            let limit = expect.map(|(_, size)| size);
            loop {
                let n = data.read(&mut buf).map_err(io_err("reading object data"))?;
                if n == 0 {
                    break;
                }
                hasher.update(&buf[..n]);
                f.write_all(&buf[..n]).map_err(io_err("writing object"))?;
                if let Some(limit) = limit {
                    if hasher.len() > limit {
                        return Err(Error::IntegrityFailed("object larger than declared".into()));
                    }
                }
            }
            f.sync_all().map_err(io_err("flushing object"))?;
            drop(f);
            let (hash, size) = hasher.finish();
            if let Some((want_hash, want_size)) = expect {
                if &hash != want_hash || size != want_size {
                    return Err(Error::IntegrityFailed(format!(
                        "object expected {want_hash} ({want_size} bytes), got {hash} ({size} bytes)"
                    )));
                }
            }
            let dest = self.object_path(&hash);
            if dest.is_file() {
                let _ = fs::remove_file(&tmp);
            } else {
                let parent = dest.parent().unwrap();
                fs::create_dir_all(parent).map_err(io_err("creating object folder"))?;
                fs::rename(&tmp, &dest).map_err(io_err("storing object"))?;
                sync_dir(parent);
            }
            Ok((hash, size))
        })();
        if result.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        result
    }

    pub fn open_object(&self, hash: &ContentHash) -> Result<File> {
        File::open(self.object_path(hash)).map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => Error::NotFound(format!("object {hash}")),
            _ => Error::io(format!("opening object {hash}"), Side, e),
        })
    }

    /// Re-read an object and confirm its bytes still match.
    pub fn verify_object(&self, entry: &FileEntry) -> Result<()> {
        let mut f = self.open_object(&entry.hash)?;
        let (hash, size) = ContentHash::of_reader(&mut f)
            .map_err(io_err(format!("reading object {}", entry.hash)))?;
        if hash != entry.hash || size != entry.size {
            return Err(Error::IntegrityFailed(format!(
                "stored copy of {} is damaged",
                entry.path
            )));
        }
        Ok(())
    }

    /// Verify every object of a checkpoint.
    pub fn verify_checkpoint_objects(&self, cp: &Checkpoint) -> Result<()> {
        cp.files.iter().try_for_each(|f| self.verify_object(f))
    }

    // ---- checkpoints --------------------------------------------------

    fn checkpoint_path(&self, id: &CheckpointId) -> PathBuf {
        self.root.join("checkpoints").join(format!("{id}.json"))
    }

    pub fn has_checkpoint(&self, id: &CheckpointId) -> bool {
        self.checkpoint_path(id).is_file()
    }

    /// Store a checkpoint after validating it and confirming every object it
    /// references is present. Idempotent.
    pub fn put_checkpoint(&self, cp: &Checkpoint) -> Result<CheckpointId> {
        cp.validate()?;
        if let Some(missing) = cp.files.iter().find(|f| !self.has_object(&f.hash)) {
            return Err(Error::IntegrityFailed(format!(
                "checkpoint references missing content for {}",
                missing.path
            )));
        }
        let id = cp.id();
        let path = self.checkpoint_path(&id);
        if !path.is_file() {
            self.write_atomic(&path, &serde_json::to_vec_pretty(cp).unwrap())?;
        }
        Ok(id)
    }

    /// Store a checkpoint whose files this vault will not hold: received as
    /// history only, from a device that no longer keeps them. The engine
    /// records it as pruned so nothing tries to read its files. Idempotent.
    pub fn put_checkpoint_metadata(&self, cp: &Checkpoint) -> Result<CheckpointId> {
        cp.validate()?;
        let id = cp.id();
        let path = self.checkpoint_path(&id);
        if !path.is_file() {
            self.write_atomic(&path, &serde_json::to_vec_pretty(cp).unwrap())?;
        }
        Ok(id)
    }

    /// Delete a checkpoint's metadata. Its objects stay until a sweep.
    pub fn delete_checkpoint(&self, id: &CheckpointId) -> Result<()> {
        match fs::remove_file(self.checkpoint_path(id)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(io_err("deleting checkpoint")(e)),
        }
    }

    /// Load a checkpoint, confirming its content still hashes to its id.
    pub fn checkpoint(&self, id: &CheckpointId) -> Result<Checkpoint> {
        let path = self.checkpoint_path(id);
        if !path.is_file() {
            return Err(Error::NotFound(format!("checkpoint {}", id.short())));
        }
        let cp: Checkpoint = read_json(&path)?;
        if &cp.id() != id {
            return Err(Error::IntegrityFailed(format!("checkpoint {} is damaged", id.short())));
        }
        Ok(cp)
    }

    /// All checkpoints in the vault. Damaged files are reported, not skipped.
    pub fn checkpoints(&self) -> Result<Vec<(CheckpointId, Checkpoint)>> {
        let mut out = Vec::new();
        let dir = self.root.join("checkpoints");
        for entry in fs::read_dir(&dir).map_err(io_err("listing checkpoints"))? {
            let entry = entry.map_err(io_err("listing checkpoints"))?;
            let name = entry.file_name();
            let Some(stem) = name.to_str().and_then(|n| n.strip_suffix(".json")) else { continue };
            let id = CheckpointId::parse(stem)?;
            let cp = self.checkpoint(&id)?;
            out.push((id, cp));
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(out)
    }

    // ---- records ------------------------------------------------------

    fn record_path(&self, kind: &str, key: &str) -> PathBuf {
        debug_assert!(is_safe_key(kind) && is_safe_key(key));
        self.root.join("records").join(kind).join(format!("{key}.json"))
    }

    pub fn put_record<T: Serialize>(&self, kind: &str, key: &str, value: &T) -> Result<()> {
        check_key(kind)?;
        check_key(key)?;
        self.write_atomic(&self.record_path(kind, key), &serde_json::to_vec_pretty(value).unwrap())
    }

    pub fn get_record<T: DeserializeOwned>(&self, kind: &str, key: &str) -> Result<Option<T>> {
        check_key(kind)?;
        check_key(key)?;
        let path = self.record_path(kind, key);
        if !path.is_file() {
            return Ok(None);
        }
        read_json(&path).map(Some)
    }

    pub fn delete_record(&self, kind: &str, key: &str) -> Result<()> {
        check_key(kind)?;
        check_key(key)?;
        let path = self.record_path(kind, key);
        match fs::remove_file(&path) {
            Ok(()) => {
                sync_dir(path.parent().unwrap());
                Ok(())
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(Error::io("deleting record", Side, e)),
        }
    }

    pub fn list_records<T: DeserializeOwned>(&self, kind: &str) -> Result<Vec<(String, T)>> {
        check_key(kind)?;
        let dir = self.root.join("records").join(kind);
        let mut out = Vec::new();
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(out),
            Err(e) => return Err(Error::io("listing records", Side, e)),
        };
        for entry in entries {
            let entry = entry.map_err(io_err("listing records"))?;
            let name = entry.file_name();
            let Some(key) = name.to_str().and_then(|n| n.strip_suffix(".json")) else { continue };
            out.push((key.to_owned(), read_json(&entry.path())?));
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(out)
    }

    // ---- locks --------------------------------------------------------

    /// Take the exclusive lock for one bundle (I10). Fails with `Busy` if
    /// another process holds it. Released on drop or process exit.
    pub fn lock(&self, name: &str) -> Result<VaultLock> {
        check_key(name)?;
        let f = File::options()
            .create(true)
            .truncate(false)
            .write(true)
            .open(self.root.join("locks").join(format!("{name}.lock")))
            .map_err(io_err("opening lock"))?;
        match f.try_lock() {
            Ok(()) => Ok(VaultLock { _file: f }),
            Err(TryLockError::WouldBlock) => Err(Error::Busy),
            Err(TryLockError::Error(e)) => Err(Error::io("taking lock", Side, e)),
        }
    }

    /// Take a shared lock. Captures and receives hold `gc` shared so that
    /// [`Vault::sweep_unreferenced`], which needs it exclusively, never runs
    /// while objects are being added.
    pub fn lock_shared(&self, name: &str) -> Result<VaultLock> {
        check_key(name)?;
        let f = File::options()
            .create(true)
            .truncate(false)
            .write(true)
            .open(self.root.join("locks").join(format!("{name}.lock")))
            .map_err(io_err("opening lock"))?;
        match f.try_lock_shared() {
            Ok(()) => Ok(VaultLock { _file: f }),
            Err(TryLockError::WouldBlock) => Err(Error::Busy),
            Err(TryLockError::Error(e)) => Err(Error::io("taking lock", Side, e)),
        }
    }

    // ---- cleanup ------------------------------------------------------

    /// Remove leftovers that no checkpoint can reference: files in `tmp/`
    /// and objects not referenced by any checkpoint. `keep` protects objects
    /// that live records still need (e.g. received data awaiting admission).
    ///
    /// Requires the caller to hold every bundle lock and the global vault
    /// lock so no capture or receive is midway. Returns bytes freed.
    pub fn sweep_unreferenced(&self, keep: &HashSet<ContentHash>) -> Result<u64> {
        let mut freed = 0u64;
        for entry in fs::read_dir(self.root.join("tmp")).map_err(io_err("listing tmp"))? {
            let entry = entry.map_err(io_err("listing tmp"))?;
            freed += entry.metadata().map(|m| m.len()).unwrap_or(0);
            let _ = fs::remove_file(entry.path());
        }
        Ok(freed + self.sweep_objects(keep, &HashSet::new())?)
    }

    /// Remove objects that no checkpoint outside `without` references and
    /// that are not in `keep`. Returns bytes freed. Callers hold the
    /// exclusive "gc" lock, so no capture or import is adding objects.
    pub fn sweep_objects(
        &self,
        keep: &HashSet<ContentHash>,
        without: &HashSet<CheckpointId>,
    ) -> Result<u64> {
        let mut referenced: HashSet<ContentHash> = keep.clone();
        for (id, cp) in self.checkpoints()? {
            if !without.contains(&id) {
                referenced.extend(cp.files.into_iter().map(|f| f.hash));
            }
        }
        let mut freed = 0u64;
        for shard in fs::read_dir(self.root.join("objects")).map_err(io_err("listing objects"))? {
            let shard = shard.map_err(io_err("listing objects"))?;
            if !shard.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            for obj in fs::read_dir(shard.path()).map_err(io_err("listing objects"))? {
                let obj = obj.map_err(io_err("listing objects"))?;
                let Some(name) = obj.file_name().to_str().map(str::to_owned) else { continue };
                let Ok(hash) = ContentHash::parse(&format!("sha256:{name}")) else { continue };
                if !referenced.contains(&hash) {
                    freed += obj.metadata().map(|m| m.len()).unwrap_or(0);
                    fs::remove_file(obj.path()).map_err(io_err("removing unreferenced object"))?;
                }
            }
        }
        Ok(freed)
    }
}

/// Held lock; released when dropped.
pub struct VaultLock {
    _file: File,
}

impl Graph for Vault {
    fn parents(&self, id: &CheckpointId) -> Option<Vec<CheckpointId>> {
        self.checkpoint(id).ok().map(|cp| cp.parents)
    }
}

fn is_safe_key(k: &str) -> bool {
    !k.is_empty()
        && k.len() <= 128
        && k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
        && !k.starts_with('.')
}

fn check_key(k: &str) -> Result<()> {
    if is_safe_key(k) {
        Ok(())
    } else {
        Err(Error::InvalidInput(format!("unsafe record key {k:?}")))
    }
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let f = File::open(path).map_err(io_err(format!("opening {}", path.display())))?;
    let mut buf = Vec::new();
    f.take(MAX_RECORD_BYTES + 1)
        .read_to_end(&mut buf)
        .map_err(io_err(format!("reading {}", path.display())))?;
    if buf.len() as u64 > MAX_RECORD_BYTES {
        return Err(Error::InvalidInput(format!("{} is too large", path.display())));
    }
    serde_json::from_slice(&buf)
        .map_err(|e| Error::IntegrityFailed(format!("{} is unreadable: {e}", path.display())))
}

#[cfg(test)]
mod tests;
