//! Replacing playable files: apply, commit and recovery (contract section 6).
//!
//! Ordering is the safety argument, so it is spelled out:
//!
//! 1. Recheck: head and playable files must equal what the user reviewed.
//! 2. Backup: the playable files must equal a stored checkpoint whose bytes
//!    re-verify from the vault. Otherwise stop; nothing was touched.
//! 3. Journal (`Prepared`): records the op id before anything is staged, so
//!    leftovers are always attributable.
//! 4. Stage every file that differs and verify the staged bytes. Still
//!    nothing playable touched.
//! 5. Journal (`Installing`): from here the files may be mixed, and the
//!    journal names the two complete states they must end in.
//! 6. Replace/remove files one at a time.
//! 7. Journal (`Committing`), verify the folder equals the target exactly.
//! 8. Commit: record the choice checkpoint if needed, move head, close the
//!    journal. Head never moves before the files verify.
//!
//! After an interruption in 6 or 7, [`Engine::recover`] either finishes the
//! install or puts the backup back. Both are the same idempotent "make the
//! folder equal checkpoint X" procedure, so recovery can itself be
//! interrupted and resumed.

use doki_core::{
    relate, BundleId, BundlePath, CaptureMode, Checkpoint, CheckpointId, CheckpointKind, Error,
    FileEntry, FilesDigest, Relation, Result, CHECKPOINT_SCHEMA_VERSION,
};
use doki_platform::SaveLocation;
use doki_vault::{now_rfc3339, random_hex};

use crate::records::{
    InstallPhase, InstallPurpose, Journal, Proposal, ProposalStatus, JOURNALS, JOURNAL_FORMAT,
    PROPOSALS,
};
use crate::{by_path, Engine, LocationProvider, Review};

/// Result of a completed [`Engine::use_checkpoint`].
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct UseOutcome {
    /// The checkpoint whose files are now playable.
    pub installed: CheckpointId,
    /// New head: `installed`, or a choice checkpoint recording what was set
    /// aside.
    pub head: CheckpointId,
    /// Verified copy of the previous playable save (`None` if the folder
    /// had no files).
    pub backup: Option<CheckpointId>,
    /// Progress set aside by this choice, still in history.
    pub set_aside: Option<CheckpointId>,
}

/// State of one path after an interruption.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileState {
    /// Matches what was being installed.
    Target,
    /// Matches the backup taken before the install.
    Backup,
    /// Absent, but expected by both the target and the backup: a replace
    /// was interrupted between removing the old file and placing the new.
    Missing,
    /// Matches neither, e.g. the emulator wrote during the install.
    Other,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RecoveryStatus {
    pub journal: Journal,
    pub files: Vec<(BundlePath, FileState)>,
    /// The folder already equals the target; recovery only needs to commit.
    pub complete: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryAction {
    /// Finish installing what the user chose.
    Finish,
    /// Put the save from before the install back.
    PutBack,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case", tag = "outcome")]
pub enum RecoveryOutcome {
    /// The install stopped before touching playable files.
    NothingChanged,
    Finished {
        head: CheckpointId,
    },
    PutBack {
        head: Option<CheckpointId>,
    },
}

impl<P: LocationProvider> Engine<P> {
    /// "Use this save": install a reviewed checkpoint (contract A0–A8).
    pub fn use_checkpoint(&self, review: &Review) -> Result<UseOutcome> {
        let _gc = self.vault.lock_shared("gc")?;
        let _lock = self.lock_bundle(&review.bundle_id)?;
        let record = self.bundle(&review.bundle_id)?;
        let location = self.provider.open(&record.location);

        // A1: recheck against the reviewed baseline (I7).
        if record.head != review.baseline_head {
            return Err(Error::LocalChangedSinceReview { new_checkpoint: None });
        }
        location.check_available()?;
        if !location.capabilities().replace.never_truncates() {
            return Err(Error::StrategyUnsupported(
                "this save location cannot replace files without risk of truncation".into(),
            ));
        }
        let live = self.scan(location.as_ref())?;
        let live_digest = (!live.is_empty()).then(|| FilesDigest::of(&live));
        if live_digest != review.baseline_files {
            let new_checkpoint = if live.is_empty() {
                None
            } else {
                Some(
                    self.capture_locked(
                        &review.bundle_id,
                        CheckpointKind::Capture,
                        CaptureMode::QuiescenceHeuristic,
                        Some("Saved automatically: the save changed after review".into()),
                    )?
                    .id()
                    .clone(),
                )
            };
            return Err(Error::LocalChangedSinceReview { new_checkpoint });
        }
        self.crash_point("use:after-recheck");

        // A2/A3: the current files must be a stored checkpoint whose bytes
        // still verify (I3). Review captured any uncaptured changes, so a
        // non-empty folder equals head here.
        let backup = if live.is_empty() {
            None
        } else {
            let head = record.head.clone().ok_or_else(|| {
                Error::BackupUnverified("the current save has no checkpoint".into())
            })?;
            let head_cp = self.vault.checkpoint(&head).map_err(backup_err)?;
            if Some(head_cp.files_digest()) != live_digest {
                return Err(Error::LocalChangedSinceReview { new_checkpoint: None });
            }
            self.vault.verify_checkpoint_objects(&head_cp).map_err(backup_err)?;
            Some(head)
        };
        self.crash_point("use:after-backup-verified");

        let incoming = self.vault.checkpoint(&review.incoming)?;
        self.vault.verify_checkpoint_objects(&incoming)?;
        let choice = match &record.head {
            Some(head) if relate(&self.vault, head, &review.incoming) != Relation::Ahead => {
                Some(Box::new(Checkpoint {
                    schema_version: CHECKPOINT_SCHEMA_VERSION,
                    bundle_id: review.bundle_id.clone(),
                    parents: vec![review.incoming.clone(), head.clone()],
                    kind: CheckpointKind::Choice,
                    files: incoming.files.clone(),
                    profile: incoming.profile.clone(),
                    origin_device: self.vault.device_id().clone(),
                    origin_device_name: self.vault.device_name().to_owned(),
                    captured_at: now_rfc3339(),
                    capture_mode: CaptureMode::Automatic,
                    note: Some(if &incoming.origin_device == self.vault.device_id() {
                        format!("Restored checkpoint {}", review.incoming.short())
                    } else {
                        format!(
                            "Chose the save from {} over {}'s",
                            incoming.origin_device_name,
                            self.vault.device_name()
                        )
                    }),
                }))
            }
            _ => None,
        };

        // A4: journal before staging.
        let mut journal = Journal {
            format: JOURNAL_FORMAT,
            op_id: random_hex(8),
            bundle_id: review.bundle_id.clone(),
            purpose: InstallPurpose::Use {
                checkpoint: review.incoming.clone(),
                choice,
                proposal: review.proposal.clone(),
            },
            target: Some(review.incoming.clone()),
            backup: backup.clone(),
            previous_head: record.head.clone(),
            phase: InstallPhase::Prepared,
            rescued: Vec::new(),
            started_at: now_rfc3339(),
        };
        self.put_journal(&journal)?;
        self.crash_point("use:after-journal");

        if let Err(e) = self.install(location.as_ref(), &mut journal) {
            if journal.phase == InstallPhase::Prepared {
                // Nothing playable was touched; tidy up and report the cause.
                let _ = self.abandon_prepared(&journal);
                return Err(e);
            }
            return Err(Error::InstallInterrupted(e.to_string()));
        }
        let head = self.commit(location.as_ref(), &journal)?.expect("using a checkpoint sets head");
        let set_aside = match &journal.purpose {
            InstallPurpose::Use { choice: Some(_), .. } => record.head.clone(),
            _ => None,
        };
        Ok(UseOutcome { installed: review.incoming.clone(), head, backup, set_aside })
    }

    fn put_journal(&self, journal: &Journal) -> Result<()> {
        self.vault.put_record(JOURNALS, journal.bundle_id.as_str(), journal)
    }

    pub(crate) fn abandon_prepared(&self, journal: &Journal) -> Result<()> {
        debug_assert_eq!(journal.phase, InstallPhase::Prepared);
        if let Ok(record) = self.bundle(&journal.bundle_id) {
            let _ = self.provider.open(&record.location).discard_staging(&journal.op_id);
        }
        self.vault.delete_record(JOURNALS, journal.bundle_id.as_str())
    }

    fn target_files(&self, journal: &Journal) -> Result<Vec<FileEntry>> {
        match &journal.target {
            Some(id) => Ok(self.vault.checkpoint(id)?.files),
            None => Ok(Vec::new()),
        }
    }

    /// Make the folder equal the journal's target (steps 4–7). Idempotent:
    /// files that already match are left alone.
    fn install(&self, location: &dyn SaveLocation, journal: &mut Journal) -> Result<()> {
        let target = self.target_files(journal)?;
        let current = self.scan(location)?;
        let current_map = by_path(&current);
        let target_map = by_path(&target);
        let to_write: Vec<&FileEntry> = target
            .iter()
            .filter(|f| current_map.get(&f.path).map(|c| c.hash != f.hash).unwrap_or(true))
            .collect();
        let to_remove: Vec<&BundlePath> =
            current.iter().map(|f| &f.path).filter(|p| !target_map.contains_key(p)).collect();

        for f in &to_write {
            let mut src = self.vault.open_object(&f.hash)?;
            location.stage(&journal.op_id, &f.path, &mut src)?;
            let mut staged = location.open_staged(&journal.op_id, &f.path)?;
            let (hash, size) = doki_core::ContentHash::of_reader(&mut staged).map_err(|e| {
                Error::io("verifying staged file", doki_core::error::StorageSide::SaveLocation, e)
            })?;
            if hash != f.hash || size != f.size {
                return Err(Error::IntegrityFailed(format!("staged copy of {} differs", f.path)));
            }
            self.crash_point("install:after-stage");
        }

        if journal.phase == InstallPhase::Prepared {
            journal.phase = InstallPhase::Installing;
            self.put_journal(journal)?;
        }
        self.crash_point("install:before-first-write");

        for f in &to_write {
            location.commit_staged(&journal.op_id, &f.path)?;
            self.crash_point("install:after-file");
        }
        for p in &to_remove {
            location.remove(p)?;
            self.crash_point("install:after-remove");
        }

        journal.phase = InstallPhase::Committing;
        self.put_journal(journal)?;
        self.crash_point("install:before-verify");
        if self.scan(location)? != target {
            return Err(Error::IntegrityFailed("installed files did not verify".into()));
        }
        Ok(())
    }

    /// Step 8. Idempotent, so recovery can repeat it.
    fn commit(
        &self,
        location: &dyn SaveLocation,
        journal: &Journal,
    ) -> Result<Option<CheckpointId>> {
        let mut record = self.bundle(&journal.bundle_id)?;
        let head = match &journal.purpose {
            InstallPurpose::Use { checkpoint, choice, proposal } => {
                let head = match choice {
                    Some(c) => self.vault.put_checkpoint(c)?,
                    None => checkpoint.clone(),
                };
                record.head = Some(head.clone());
                self.save_bundle(&record)?;
                if let Some(key) = proposal {
                    if let Some(mut p) = self.vault.get_record::<Proposal>(PROPOSALS, key)? {
                        p.status = ProposalStatus::Applied;
                        self.vault.put_record(PROPOSALS, key, &p)?;
                    }
                }
                Some(head)
            }
            InstallPurpose::PutBack => {
                record.head = journal.previous_head.clone();
                self.save_bundle(&record)?;
                record.head.clone()
            }
        };
        self.crash_point("commit:after-head");
        // Leftover staging is harmless and excluded from the bundle; do not
        // fail a verified install over it.
        let _ = location.discard_staging(&journal.op_id);
        self.vault.delete_record(JOURNALS, journal.bundle_id.as_str())?;
        Ok(head)
    }

    /// Describe an interrupted install, if one is waiting.
    pub fn recovery_status(&self, bundle: &BundleId) -> Result<Option<RecoveryStatus>> {
        let Some(journal) = self.journal(bundle)? else { return Ok(None) };
        let record = self.bundle(bundle)?;
        let location = self.provider.open(&record.location);
        let live = self.scan(location.as_ref())?;
        let target = self.target_files(&journal)?;
        let backup = match &journal.backup {
            Some(id) => self.vault.checkpoint(id)?.files,
            None => Vec::new(),
        };
        let (l, t, b) = (by_path(&live), by_path(&target), by_path(&backup));
        let mut paths: Vec<&BundlePath> =
            l.keys().chain(t.keys()).chain(b.keys()).copied().collect();
        paths.sort();
        paths.dedup();
        let hash_of = |m: &std::collections::BTreeMap<&BundlePath, &FileEntry>, p: &BundlePath| {
            m.get(p).map(|f| f.hash.clone())
        };
        let files = paths
            .into_iter()
            .map(|p| {
                let now = hash_of(&l, p);
                let state = if now == hash_of(&t, p) {
                    FileState::Target
                } else if now == hash_of(&b, p) {
                    FileState::Backup
                } else if now.is_none() {
                    FileState::Missing
                } else {
                    FileState::Other
                };
                (p.clone(), state)
            })
            .collect();
        Ok(Some(RecoveryStatus { complete: live == target, journal, files }))
    }

    /// Resolve an interrupted install (contract 6.1).
    ///
    /// If the folder already equals the target, the install is committed
    /// whatever `action` says. If any file matches neither the target nor
    /// the backup, that state is captured into history first so nothing is
    /// lost. Then the folder is made to equal the target (`Finish`) or the
    /// backup (`PutBack`).
    pub fn recover(&self, bundle: &BundleId, action: RecoveryAction) -> Result<RecoveryOutcome> {
        let _gc = self.vault.lock_shared("gc")?;
        let _lock = self.vault.lock(&format!("bundle-{bundle}"))?;
        let Some(mut journal) = self.journal(bundle)? else {
            return Err(Error::NotFound("no interrupted install for this game".into()));
        };
        if journal.phase == InstallPhase::Prepared {
            self.abandon_prepared(&journal)?;
            return Ok(RecoveryOutcome::NothingChanged);
        }
        let record = self.bundle(bundle)?;
        let location = self.provider.open(&record.location);
        location.check_available().map_err(|e| Error::RecoveryFailed(e.to_string()))?;
        let status = self.recovery_status(bundle)?.expect("journal exists");
        let fail = |e: Error| Error::RecoveryFailed(e.to_string());

        if status.complete {
            let head = self.commit(location.as_ref(), &journal).map_err(fail)?;
            return Ok(outcome(&journal, head));
        }

        if status.files.iter().any(|(_, s)| *s == FileState::Other) {
            let files = self.copy_stable(location.as_ref()).map_err(fail)?;
            if !files.is_empty() {
                let rescued = Checkpoint {
                    schema_version: CHECKPOINT_SCHEMA_VERSION,
                    bundle_id: bundle.clone(),
                    parents: journal
                        .backup
                        .iter()
                        .chain(journal.previous_head.iter())
                        .take(1)
                        .cloned()
                        .collect(),
                    kind: CheckpointKind::Backup,
                    files,
                    profile: record.profile.clone(),
                    origin_device: self.vault.device_id().clone(),
                    origin_device_name: self.vault.device_name().to_owned(),
                    captured_at: now_rfc3339(),
                    capture_mode: CaptureMode::Automatic,
                    note: Some("Found in the save folder after an interrupted install".into()),
                };
                let id = self.vault.put_checkpoint(&rescued).map_err(fail)?;
                if !journal.rescued.contains(&id) {
                    journal.rescued.push(id);
                    self.put_journal(&journal).map_err(fail)?;
                }
            }
        }

        if action == RecoveryAction::PutBack && journal.purpose != InstallPurpose::PutBack {
            journal.purpose = InstallPurpose::PutBack;
            journal.target = journal.backup.clone();
            journal.phase = InstallPhase::Installing;
            self.put_journal(&journal).map_err(fail)?;
        }
        self.install(location.as_ref(), &mut journal).map_err(fail)?;
        let head = self.commit(location.as_ref(), &journal).map_err(fail)?;
        Ok(outcome(&journal, head))
    }
}

fn outcome(journal: &Journal, head: Option<CheckpointId>) -> RecoveryOutcome {
    match (&journal.purpose, head) {
        (InstallPurpose::Use { .. }, Some(head)) => RecoveryOutcome::Finished { head },
        (_, head) => RecoveryOutcome::PutBack { head },
    }
}

fn backup_err(e: Error) -> Error {
    Error::BackupUnverified(e.to_string())
}
