//! Doki safety engine.
//!
//! Implements the operations in `docs/spec/safety-contract.md`:
//!
//! - [`Engine::capture`]: copy the playable save into the vault as a checkpoint.
//! - [`Engine::import_package`] / [`Engine::export_package`]: move checkpoints
//!   between devices. Receiving only ever writes the vault (I2).
//! - [`Engine::review`] then [`Engine::use_checkpoint`]: replace the playable
//!   save after a recheck and a verified backup, under a durable journal.
//! - [`Engine::recover`]: finish or undo an interrupted install.
//! - [`Engine::keep_alongside`] / [`Engine::dismiss`]: the other review outcomes.
//!
//! Every operation that touches one bundle takes that bundle's lock, and
//! refuses to run while an install journal is open for it.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use doki_core::hash::ContentHash;
use doki_core::{
    BundleId, BundlePath, CaptureMode, Checkpoint, CheckpointId, CheckpointKind, Error, FileEntry,
    FilesDigest, ProfileRef, Result, CHECKPOINT_SCHEMA_VERSION,
};
use doki_platform::local::LocalDir;
use doki_platform::SaveLocation;
use doki_vault::{now_rfc3339, random_hex, Vault, VaultLock};

mod cleanup;
mod install;
mod manage;
mod package;
mod proposals;
pub mod records;
mod systems;

pub use cleanup::{Removal, SettingsChanges, KEEP_NEWEST};
pub use install::{FileState, RecoveryAction, RecoveryOutcome, RecoveryStatus, UseOutcome};
pub use manage::{AutoCheck, AutoOutcome, BundleChanges};
pub use package::{ImportSummary, PACKAGE_FORMAT};
pub use proposals::{Review, ReviewItem};
pub use records::{
    BundleRecord, ForgottenGame, Journal, MemberKind, MemberRef, Proposal, ProposalStatus,
    RemoteSystem, Settings, SystemRecord, UnmappedBundle,
};
pub use systems::SystemChanges;

use records::{BUNDLES, JOURNALS};

/// Profile used until emulator profiles exist: every regular file in the
/// chosen folder is part of the bundle.
pub fn generic_folder_profile() -> ProfileRef {
    ProfileRef { id: "generic-folder".into(), version: 1 }
}

/// Opens the save location recorded for a bundle.
pub trait LocationProvider {
    fn open<'a>(&'a self, location: &str) -> Box<dyn SaveLocation + 'a>;
}

/// Desktop folders.
pub struct LocalProvider;

impl LocationProvider for LocalProvider {
    fn open<'a>(&'a self, location: &str) -> Box<dyn SaveLocation + 'a> {
        Box::new(LocalDir::new(location))
    }
}

/// Test hook called at named points between steps. Tests panic inside it to
/// simulate the process dying there: no cleanup code runs, and the OS
/// releases the vault locks as it would for a killed process.
pub type CrashHook = Box<dyn Fn(&str)>;

/// Result of [`Engine::capture`].
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case", tag = "outcome", content = "checkpoint")]
pub enum CaptureOutcome {
    Created(CheckpointId),
    /// The playable files equal the current head; nothing new was stored.
    Unchanged(CheckpointId),
}

impl CaptureOutcome {
    pub fn id(&self) -> &CheckpointId {
        match self {
            CaptureOutcome::Created(id) | CaptureOutcome::Unchanged(id) => id,
        }
    }
}

/// One checkpoint as shown in a bundle's history.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HistoryEntry {
    pub id: CheckpointId,
    pub checkpoint: Checkpoint,
    pub is_head: bool,
    /// No other checkpoint of this bundle builds on it.
    pub is_tip: bool,
    /// Progress a choice set aside: kept in history, not continued.
    pub set_aside: bool,
    pub pinned: bool,
    /// This device removed the checkpoint's files (deleted or past the
    /// retention time). It stays in history for ancestry only and cannot be
    /// used or restored here.
    pub files_removed: bool,
}

pub struct Engine<P: LocationProvider = LocalProvider> {
    vault: Vault,
    provider: P,
    crash_hook: Option<CrashHook>,
}

impl Engine<LocalProvider> {
    pub fn open(vault_root: impl Into<PathBuf>) -> Result<Self> {
        Ok(Engine::with_provider(Vault::open(vault_root)?, LocalProvider))
    }
}

impl<P: LocationProvider> Engine<P> {
    pub fn with_provider(vault: Vault, provider: P) -> Self {
        Engine { vault, provider, crash_hook: None }
    }

    pub fn vault(&self) -> &Vault {
        &self.vault
    }

    /// Install a crash hook (tests only).
    pub fn set_crash_hook(&mut self, hook: Option<CrashHook>) {
        self.crash_hook = hook;
    }

    pub(crate) fn crash_point(&self, name: &str) {
        if let Some(hook) = &self.crash_hook {
            hook(name);
        }
    }

    // ---- bundles ------------------------------------------------------

    /// Set up a game save on this device.
    ///
    /// Pass `bundle_id` to link the save to a game already set up on another
    /// device (its id appears in received packages); otherwise a new id is
    /// created. Registering never reads or changes save files beyond
    /// checking the folder exists.
    pub fn add_bundle(
        &self,
        name: &str,
        location: &str,
        bundle_id: Option<BundleId>,
    ) -> Result<BundleRecord> {
        if name.trim().is_empty() {
            return Err(Error::InvalidInput("a game name is required".into()));
        }
        let location = self.check_location(location, None)?;
        for existing in self.bundles()? {
            if Some(&existing.bundle_id) == bundle_id.as_ref() {
                return Err(Error::InvalidInput(format!(
                    "this game is already set up as {}",
                    existing.name
                )));
            }
        }
        let record = BundleRecord {
            bundle_id: bundle_id.unwrap_or(BundleId::parse(&random_hex(8))?),
            name: name.trim().to_owned(),
            location,
            profile: generic_folder_profile(),
            head: None,
            pins: Default::default(),
            created_at: now_rfc3339(),
            auto_capture: true,
            member: None,
        };
        self.vault.put_record(BUNDLES, record.bundle_id.as_str(), &record)?;
        self.vault.delete_record(records::UNMAPPED, record.bundle_id.as_str())?;
        Ok(record)
    }

    pub fn bundles(&self) -> Result<Vec<BundleRecord>> {
        Ok(self.vault.list_records(BUNDLES)?.into_iter().map(|(_, b)| b).collect())
    }

    pub fn bundle(&self, id: &BundleId) -> Result<BundleRecord> {
        self.vault
            .get_record(BUNDLES, id.as_str())?
            .ok_or_else(|| Error::NotFound(format!("game {}", id.short())))
    }

    /// Find a bundle by id prefix or case-insensitive name.
    pub fn find_bundle(&self, query: &str) -> Result<BundleRecord> {
        let all = self.bundles()?;
        let q = query.to_lowercase();
        let matches: Vec<_> = all
            .into_iter()
            .filter(|b| b.bundle_id.as_str().starts_with(&q) || b.name.to_lowercase() == q)
            .collect();
        match matches.len() {
            1 => Ok(matches.into_iter().next().unwrap()),
            0 => Err(Error::NotFound(format!("no game matches {query:?}"))),
            _ => Err(Error::InvalidInput(format!("{query:?} matches several games"))),
        }
    }

    /// The save location of a set-up game: its own folder, or its part of a
    /// whole-system folder.
    pub(crate) fn open_bundle(&self, record: &BundleRecord) -> Box<dyn SaveLocation + '_> {
        let folder = self.provider.open(&record.location);
        match &record.member {
            Some(m) => Box::new(doki_platform::member::MemberLocation::new(folder, m.member())),
            None => folder,
        }
    }

    pub(crate) fn save_bundle(&self, record: &BundleRecord) -> Result<()> {
        self.vault.put_record(BUNDLES, record.bundle_id.as_str(), record)
    }

    /// Lock a bundle and confirm no install is waiting for recovery.
    ///
    /// A journal still in the `Prepared` phase means an install stopped
    /// before touching playable files; it is cleaned up here. Any later
    /// phase requires [`Engine::recover`].
    pub(crate) fn lock_bundle(&self, id: &BundleId) -> Result<VaultLock> {
        let lock = self.vault.lock(&format!("bundle-{id}"))?;
        if let Some(journal) = self.journal(id)? {
            if journal.phase != records::InstallPhase::Prepared {
                return Err(Error::RecoveryRequired);
            }
            self.abandon_prepared(&journal)?;
        }
        Ok(lock)
    }

    pub fn journal(&self, id: &BundleId) -> Result<Option<Journal>> {
        self.vault.get_record(JOURNALS, id.as_str())
    }

    // ---- reading the save folder -------------------------------------

    /// Hash every file in the save folder without storing anything.
    pub(crate) fn scan(&self, location: &dyn SaveLocation) -> Result<Vec<FileEntry>> {
        let mut files = Vec::new();
        for path in location.list()? {
            let mut r = location.open(&path)?;
            let (hash, size) = ContentHash::of_reader(&mut r).map_err(|e| {
                Error::io(format!("reading {path}"), doki_core::error::StorageSide::SaveLocation, e)
            })?;
            files.push(FileEntry { path, size, hash });
        }
        Ok(files)
    }

    /// Copy every file in the save folder into the vault, then re-read the
    /// folder and require it to be unchanged (contract C4/C5).
    pub(crate) fn copy_stable(&self, location: &dyn SaveLocation) -> Result<Vec<FileEntry>> {
        let mut files = Vec::new();
        for path in location.list()? {
            let mut r = location.open(&path)?;
            let (hash, size) = self.vault.put_object(&mut r, None)?;
            files.push(FileEntry { path, size, hash });
        }
        if self.scan(location)? != files {
            return Err(Error::SourceChanging);
        }
        Ok(files)
    }

    // ---- capture ------------------------------------------------------

    /// Capture the playable save as a new checkpoint (contract section 4).
    ///
    /// Never writes the save folder. A missing folder is `SourceUnavailable`
    /// and an empty one is `BundleIncomplete`; neither produces a checkpoint.
    pub fn capture(
        &self,
        bundle: &BundleId,
        mode: CaptureMode,
        note: Option<String>,
    ) -> Result<CaptureOutcome> {
        let _gc = self.vault.lock_shared("gc")?;
        let _lock = self.lock_bundle(bundle)?;
        self.capture_locked(bundle, CheckpointKind::Capture, mode, note)
    }

    pub(crate) fn capture_locked(
        &self,
        bundle: &BundleId,
        kind: CheckpointKind,
        mode: CaptureMode,
        note: Option<String>,
    ) -> Result<CaptureOutcome> {
        let mut record = self.bundle(bundle)?;
        let location = self.open_bundle(&record);
        location.check_available()?;
        let files = self.copy_stable(location.as_ref())?;
        if files.is_empty() {
            return Err(Error::BundleIncomplete(
                "the save folder has no files; nothing was captured".into(),
            ));
        }
        if let Some(head) = &record.head {
            if self.vault.checkpoint(head)?.files_digest() == FilesDigest::of(&files) {
                return Ok(CaptureOutcome::Unchanged(head.clone()));
            }
        }
        let parents: Vec<CheckpointId> = record.head.iter().cloned().collect();
        // A crash after storing a checkpoint but before moving head leaves an
        // identical child of head; adopt it instead of creating a twin tip.
        let digest = FilesDigest::of(&files);
        if let Some((id, _)) = self.vault.checkpoints()?.into_iter().find(|(_, cp)| {
            &cp.bundle_id == bundle
                && cp.parents == parents
                && cp.kind == kind
                && &cp.origin_device == self.vault.device_id()
                && cp.files_digest() == digest
        }) {
            record.head = Some(id.clone());
            self.save_bundle(&record)?;
            return Ok(CaptureOutcome::Created(id));
        }
        let cp = Checkpoint {
            schema_version: CHECKPOINT_SCHEMA_VERSION,
            bundle_id: bundle.clone(),
            parents,
            kind,
            files,
            profile: record.profile.clone(),
            origin_device: self.vault.device_id().clone(),
            origin_device_name: self.vault.device_name().to_owned(),
            captured_at: now_rfc3339(),
            capture_mode: mode,
            note,
        };
        let id = self.vault.put_checkpoint(&cp)?;
        self.crash_point("capture:after-checkpoint");
        record.head = Some(id.clone());
        self.save_bundle(&record)?;
        Ok(CaptureOutcome::Created(id))
    }

    /// Whether the playable files differ from the head checkpoint.
    /// `None` when the folder cannot be read.
    pub fn has_uncaptured_changes(&self, bundle: &BundleId) -> Result<bool> {
        let record = self.bundle(bundle)?;
        let location = self.open_bundle(&record);
        let files = self.scan(location.as_ref())?;
        Ok(match &record.head {
            Some(head) => self.vault.checkpoint(head)?.files_digest() != FilesDigest::of(&files),
            None => !files.is_empty(),
        })
    }

    // ---- history ------------------------------------------------------

    /// Checkpoints of a bundle, oldest first by ancestry. Capture times only
    /// break ties for display; they never decide anything.
    pub fn history(&self, bundle: &BundleId) -> Result<Vec<HistoryEntry>> {
        let record = self.bundle(bundle)?;
        let pruned = self.pruned_ids()?;
        let mut cps: BTreeMap<CheckpointId, Checkpoint> = self
            .vault
            .checkpoints()?
            .into_iter()
            .filter(|(_, cp)| &cp.bundle_id == bundle)
            .collect();
        let has_children: HashSet<CheckpointId> =
            cps.values().flat_map(|cp| cp.parents.iter().cloned()).collect();
        let set_aside: HashSet<CheckpointId> = cps
            .values()
            .filter(|cp| cp.kind == CheckpointKind::Choice)
            .flat_map(|cp| cp.parents.iter().skip(1).cloned())
            .collect();

        // Kahn's algorithm; ready set ordered by capture time for display.
        let mut indegree: HashMap<CheckpointId, usize> = cps
            .iter()
            .map(|(id, cp)| {
                (id.clone(), cp.parents.iter().filter(|p| cps.contains_key(*p)).count())
            })
            .collect();
        let mut out = Vec::with_capacity(cps.len());
        while !indegree.is_empty() {
            let next = indegree
                .iter()
                .filter(|(_, d)| **d == 0)
                .map(|(id, _)| id.clone())
                .min_by(|a, b| cps[a].captured_at.cmp(&cps[b].captured_at).then(a.cmp(b)))
                .expect("checkpoint ids are content hashes, so history has no cycles");
            indegree.remove(&next);
            for (id, d) in indegree.iter_mut() {
                if cps[id].parents.contains(&next) {
                    *d -= 1;
                }
            }
            let cp = cps.remove(&next).unwrap();
            out.push(HistoryEntry {
                is_head: record.head.as_ref() == Some(&next),
                is_tip: !has_children.contains(&next),
                set_aside: set_aside.contains(&next),
                pinned: record.pins.contains(&next),
                files_removed: pruned.contains(&next),
                id: next,
                checkpoint: cp,
            });
        }
        Ok(out)
    }

    /// Resolve a checkpoint id prefix within a bundle's history.
    pub fn find_checkpoint(&self, bundle: &BundleId, prefix: &str) -> Result<CheckpointId> {
        let matches: Vec<_> = self
            .history(bundle)?
            .into_iter()
            .filter(|e| e.id.as_str().starts_with(prefix))
            .map(|e| e.id)
            .collect();
        match matches.len() {
            1 => Ok(matches.into_iter().next().unwrap()),
            0 => Err(Error::NotFound(format!("no checkpoint starts with {prefix:?}"))),
            _ => Err(Error::InvalidInput(format!("{prefix:?} matches several checkpoints"))),
        }
    }

    pub fn set_pinned(&self, bundle: &BundleId, cp: &CheckpointId, pinned: bool) -> Result<()> {
        let _lock = self.vault.lock(&format!("bundle-{bundle}"))?;
        let mut record = self.bundle(bundle)?;
        let found = self.vault.checkpoint(cp)?;
        if &found.bundle_id != bundle {
            return Err(Error::InvalidInput("that checkpoint belongs to another game".into()));
        }
        if pinned {
            record.pins.insert(cp.clone());
        } else {
            record.pins.remove(cp);
        }
        self.save_bundle(&record)
    }

    /// Remove temporary files and content no checkpoint references, e.g.
    /// left by a capture that was interrupted. Never removes content of any
    /// checkpoint. Returns bytes freed.
    pub fn clean_leftovers(&self) -> Result<u64> {
        let _gc = self.vault.lock("gc")?;
        let mut locks = Vec::new();
        for b in self.bundles()? {
            locks.push(self.vault.lock(&format!("bundle-{}", b.bundle_id))?);
        }
        self.vault.sweep_unreferenced(&HashSet::new())
    }

    /// Write a checkpoint's files into a plain folder, outside Doki and the
    /// emulator, as an independent copy. The destination must not exist.
    pub fn export_checkpoint_files(&self, cp: &CheckpointId, dest: &Path) -> Result<()> {
        if dest.exists() {
            return Err(Error::InvalidInput(format!("{} already exists", dest.display())));
        }
        if self.is_pruned(cp)? {
            return Err(files_removed());
        }
        let checkpoint = self.vault.checkpoint(cp)?;
        self.vault.verify_checkpoint_objects(&checkpoint)?;
        let side = doki_core::error::StorageSide::SaveLocation;
        for f in &checkpoint.files {
            let out = f.path.segments().fold(dest.to_path_buf(), |p, s| p.join(s));
            std::fs::create_dir_all(out.parent().unwrap())
                .map_err(|e| Error::io("creating export folder", side, e))?;
            let mut src = self.vault.open_object(&f.hash)?;
            let mut dst = std::fs::File::create(&out)
                .map_err(|e| Error::io(format!("writing {}", f.path), side, e))?;
            std::io::copy(&mut src, &mut dst)
                .and_then(|_| dst.sync_all())
                .map_err(|e| Error::io(format!("writing {}", f.path), side, e))?;
        }
        Ok(())
    }
}

/// The error for using a checkpoint whose files this device removed.
pub(crate) fn files_removed() -> Error {
    Error::NotFound("the files of this checkpoint were removed from this device".into())
}

fn normalize_location(location: &str) -> String {
    std::fs::canonicalize(location)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| location.to_owned())
}

fn is_within(inner: &Path, outer: &Path) -> bool {
    match (std::fs::canonicalize(inner), std::fs::canonicalize(outer)) {
        (Ok(i), Ok(o)) => i.starts_with(o),
        _ => false,
    }
}

/// Map of files by path, for comparisons.
pub(crate) fn by_path(files: &[FileEntry]) -> BTreeMap<&BundlePath, &FileEntry> {
    files.iter().map(|f| (&f.path, f)).collect()
}

#[cfg(test)]
mod tests;
