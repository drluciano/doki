//! Handoff packages: the first transport.
//!
//! A package is one file carrying checkpoints, their contents and "offers"
//! (the sender's current save for each game). Users move it however they
//! like (USB, a shared folder, a chat attachment). The self-hostable relay
//! will carry the same index and objects over the network.
//!
//! Format 1, all integers little-endian:
//!
//! ```text
//! b"DOKIPKG\0"            8-byte magic
//! u32  format             = 1
//! u64  index_len          bytes of the JSON index that follows
//! [u8; index_len]         PackageIndex as JSON
//! object bytes            each object's bytes, in index order, back to back
//! ```
//!
//! Importing verifies every object's hash and size and every checkpoint's
//! structure before anything becomes visible, and only ever writes the
//! vault (I2). A truncated or tampered package is rejected without leaving
//! partial checkpoints (I8). Packages are not encrypted or signed yet: do
//! not send them over channels you do not trust (see S01).

use std::collections::{BTreeMap, HashSet};
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::Path;

use doki_core::error::StorageSide;
use doki_core::{BundleId, Checkpoint, CheckpointId, ContentHash, DeviceId, Error, Result};
use doki_vault::now_rfc3339;
use serde::{Deserialize, Serialize};

use crate::{Engine, LocationProvider};

pub const PACKAGE_FORMAT: u32 = 1;
const MAGIC: &[u8; 8] = b"DOKIPKG\0";
const MAX_INDEX_BYTES: u64 = 64 * 1024 * 1024;
const MAX_ENTRIES: usize = 100_000;
const MAX_TOTAL_BYTES: u64 = 16 * 1024 * 1024 * 1024;

#[derive(Debug, Serialize, Deserialize)]
struct PackageIndex {
    format: u32,
    from_device: String,
    from_device_id: DeviceId,
    created_at: String,
    offers: Vec<Offer>,
    checkpoints: Vec<Checkpoint>,
    objects: Vec<ObjectRef>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Offer {
    bundle_id: BundleId,
    checkpoint: CheckpointId,
    game_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ObjectRef {
    hash: ContentHash,
    size: u64,
}

/// What an import added.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ImportSummary {
    pub from_device: String,
    pub checkpoints_added: usize,
    pub new_proposals: Vec<(BundleId, CheckpointId)>,
    /// Offers for games not set up on this device.
    pub unmapped: Vec<(BundleId, String)>,
}

fn pkg_io(context: &str) -> impl FnOnce(io::Error) -> Error + '_ {
    move |e| Error::io(context, StorageSide::Vault, e)
}

impl<P: LocationProvider> Engine<P> {
    /// Write a package offering this device's current save of each listed
    /// game, with its full history. `dest` must not exist; the file appears
    /// only once complete.
    pub fn export_package(&self, bundles: &[BundleId], dest: &Path) -> Result<usize> {
        if dest.exists() {
            return Err(Error::InvalidInput(format!("{} already exists", dest.display())));
        }
        let all = self.vault.checkpoints()?;
        let mut offers = Vec::new();
        let mut checkpoints = Vec::new();
        for bundle in bundles {
            let record = self.bundle(bundle)?;
            let Some(head) = record.head.clone() else {
                return Err(Error::InvalidInput(format!(
                    "{} has no checkpoint yet; capture one first",
                    record.name
                )));
            };
            offers.push(Offer {
                bundle_id: bundle.clone(),
                checkpoint: head,
                game_name: record.name,
            });
            checkpoints.extend(
                all.iter().filter(|(_, cp)| &cp.bundle_id == bundle).map(|(_, cp)| cp.clone()),
            );
        }
        let mut objects: BTreeMap<ContentHash, u64> = BTreeMap::new();
        for cp in &checkpoints {
            self.vault.verify_checkpoint_objects(cp)?;
            for f in &cp.files {
                objects.insert(f.hash.clone(), f.size);
            }
        }
        let index = PackageIndex {
            format: PACKAGE_FORMAT,
            from_device: self.vault.device_name().to_owned(),
            from_device_id: self.vault.device_id().clone(),
            created_at: now_rfc3339(),
            offers,
            checkpoints,
            objects: objects.into_iter().map(|(hash, size)| ObjectRef { hash, size }).collect(),
        };
        let index_bytes = serde_json::to_vec(&index).expect("index serializes");

        let partial = dest.with_extension("partial");
        let result = (|| -> Result<()> {
            let mut out =
                BufWriter::new(File::create(&partial).map_err(pkg_io("creating package"))?);
            out.write_all(MAGIC).map_err(pkg_io("writing package"))?;
            out.write_all(&PACKAGE_FORMAT.to_le_bytes()).map_err(pkg_io("writing package"))?;
            out.write_all(&(index_bytes.len() as u64).to_le_bytes())
                .map_err(pkg_io("writing package"))?;
            out.write_all(&index_bytes).map_err(pkg_io("writing package"))?;
            for obj in &index.objects {
                let mut f = self.vault.open_object(&obj.hash)?;
                io::copy(&mut f, &mut out).map_err(pkg_io("writing package"))?;
            }
            let f = out
                .into_inner()
                .map_err(|e| Error::io("writing package", StorageSide::Vault, e.into_error()))?;
            f.sync_all().map_err(pkg_io("flushing package"))?;
            fs::rename(&partial, dest).map_err(pkg_io("finishing package"))?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&partial);
        }
        result.map(|_| index.checkpoints.len())
    }

    /// Admit a package into the vault and record its offers as incoming
    /// progress. Never writes any save folder.
    pub fn import_package(&self, src: &Path) -> Result<ImportSummary> {
        let _gc = self.vault.lock_shared("gc")?;
        let mut r = BufReader::new(File::open(src).map_err(pkg_io("opening package"))?);
        let mut magic = [0u8; 8];
        r.read_exact(&mut magic).map_err(|_| Error::InvalidInput("not a Doki package".into()))?;
        if &magic != MAGIC {
            return Err(Error::InvalidInput("not a Doki package".into()));
        }
        let mut u32b = [0u8; 4];
        let mut u64b = [0u8; 8];
        r.read_exact(&mut u32b).map_err(truncated)?;
        let format = u32::from_le_bytes(u32b);
        if format != PACKAGE_FORMAT {
            return Err(Error::UnsupportedVersion(format!("package format {format}")));
        }
        r.read_exact(&mut u64b).map_err(truncated)?;
        let index_len = u64::from_le_bytes(u64b);
        if index_len > MAX_INDEX_BYTES {
            return Err(Error::InvalidInput("package index is too large".into()));
        }
        let mut index_bytes = vec![0u8; index_len as usize];
        r.read_exact(&mut index_bytes).map_err(truncated)?;
        let index: PackageIndex = serde_json::from_slice(&index_bytes)
            .map_err(|e| Error::IntegrityFailed(format!("package index is unreadable: {e}")))?;

        // Validate everything declared before storing anything.
        if index.format != PACKAGE_FORMAT {
            return Err(Error::UnsupportedVersion(format!(
                "package index format {}",
                index.format
            )));
        }
        if index.objects.len() > MAX_ENTRIES || index.checkpoints.len() > MAX_ENTRIES {
            return Err(Error::InvalidInput("package has too many entries".into()));
        }
        let total: u64 =
            index.objects.iter().map(|o| o.size).fold(0u64, |a, b| a.saturating_add(b));
        if total > MAX_TOTAL_BYTES {
            return Err(Error::InvalidInput("package is too large".into()));
        }
        let declared: HashSet<&ContentHash> = index.objects.iter().map(|o| &o.hash).collect();
        for cp in &index.checkpoints {
            cp.validate()?;
            for f in &cp.files {
                if !declared.contains(&f.hash) && !self.vault.has_object(&f.hash) {
                    return Err(Error::IntegrityFailed(format!(
                        "package is missing content for {}",
                        f.path
                    )));
                }
            }
        }
        let ids: HashSet<CheckpointId> = index.checkpoints.iter().map(|c| c.id()).collect();
        for offer in &index.offers {
            if !ids.contains(&offer.checkpoint) && !self.vault.has_checkpoint(&offer.checkpoint) {
                return Err(Error::IntegrityFailed(
                    "package offers a checkpoint it does not carry".into(),
                ));
            }
        }

        // Objects: each verified against its declared hash and size.
        for obj in &index.objects {
            let mut limited = (&mut r).take(obj.size);
            if self.vault.has_object(&obj.hash) {
                let n = io::copy(&mut limited, &mut io::sink()).map_err(truncated)?;
                if n != obj.size {
                    return Err(truncated(io::ErrorKind::UnexpectedEof.into()));
                }
            } else {
                self.vault.put_object(&mut limited, Some((&obj.hash, obj.size)))?;
            }
        }
        if r.read(&mut [0u8; 1]).map_err(pkg_io("reading package"))? != 0 {
            return Err(Error::IntegrityFailed("package has unexpected trailing data".into()));
        }

        let mut summary =
            ImportSummary { from_device: index.from_device.clone(), ..Default::default() };
        for cp in &index.checkpoints {
            if !self.vault.has_checkpoint(&cp.id()) {
                self.vault.put_checkpoint(cp)?;
                summary.checkpoints_added += 1;
            }
        }
        if &index.from_device_id == self.vault.device_id() {
            return Ok(summary); // our own package: history only, nothing incoming
        }
        for offer in &index.offers {
            let cp = self.vault.checkpoint(&offer.checkpoint)?;
            if cp.bundle_id != offer.bundle_id {
                return Err(Error::IntegrityFailed("offer names the wrong game".into()));
            }
            let mapped = self.bundle(&offer.bundle_id).is_ok();
            let _lock = if mapped {
                Some(self.vault.lock(&format!("bundle-{}", offer.bundle_id))?)
            } else {
                None
            };
            let created = self.offer(
                &offer.bundle_id,
                &offer.checkpoint,
                &index.from_device,
                &offer.game_name,
            )?;
            if created && mapped {
                summary.new_proposals.push((offer.bundle_id.clone(), offer.checkpoint.clone()));
            }
            if !mapped {
                summary.unmapped.push((offer.bundle_id.clone(), offer.game_name.clone()));
            }
        }
        Ok(summary)
    }
}

fn truncated(_: io::Error) -> Error {
    Error::IntegrityFailed("package is truncated".into())
}
