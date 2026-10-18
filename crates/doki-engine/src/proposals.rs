//! Incoming progress: offers, review and the non-installing outcomes.

use doki_core::{
    relate, BundleId, CaptureMode, Checkpoint, CheckpointId, CheckpointKind, Error, FilesDigest,
    Relation, Result, CHECKPOINT_SCHEMA_VERSION,
};
use doki_vault::now_rfc3339;

use crate::records::{Proposal, ProposalStatus, UnmappedBundle, PROPOSALS, UNMAPPED};
use crate::{CaptureOutcome, Engine, LocationProvider};

/// Everything the user needs to decide, and the exact state the decision is
/// bound to (I7). [`Engine::use_checkpoint`] refuses to act if the head or
/// the playable files no longer match `baseline_*`, so a review passed back
/// in from a client cannot bypass the recheck.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Review {
    pub bundle_id: BundleId,
    pub incoming: CheckpointId,
    pub incoming_checkpoint: Checkpoint,
    /// Head at review time.
    pub baseline_head: Option<CheckpointId>,
    /// Playable files at review time; `None` when the folder had no files.
    pub baseline_files: Option<FilesDigest>,
    /// How the incoming checkpoint relates to the baseline head. `None` when
    /// this device has no checkpoint of the game yet.
    pub relation: Option<Relation>,
    /// Set when uncaptured local changes were found and saved as a
    /// checkpoint before reviewing, so they cannot be lost.
    pub captured_local_changes: Option<CheckpointId>,
    /// Proposal key when reviewing incoming progress.
    pub proposal: Option<String>,
}

impl Review {
    /// Both devices made progress the other does not have.
    pub fn is_divergent(&self) -> bool {
        matches!(
            self.relation,
            Some(Relation::Divergent { .. } | Relation::Unrelated | Relation::Unknown { .. })
        )
    }
}

/// A row in the Incoming list.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReviewItem {
    pub proposal: Proposal,
    pub checkpoint: Checkpoint,
    pub relation: Option<Relation>,
}

impl<P: LocationProvider> Engine<P> {
    /// Record that another device offered `checkpoint` for `bundle`. The
    /// checkpoint must already be in the vault. Returns whether a new
    /// proposal was created. Never touches playable files.
    pub(crate) fn offer(
        &self,
        bundle: &BundleId,
        checkpoint: &CheckpointId,
        from_device: &str,
        name_hint: &str,
    ) -> Result<bool> {
        let key = Proposal::key(bundle, checkpoint);
        if self.vault.get_record::<Proposal>(PROPOSALS, &key)?.is_some() {
            return Ok(false); // duplicate delivery
        }
        match self
            .vault
            .get_record::<crate::BundleRecord>(crate::records::BUNDLES, bundle.as_str())?
        {
            Some(mut record) => {
                if let Some(head) = record.head.clone() {
                    match relate(&self.vault, &head, checkpoint) {
                        Relation::Same | Relation::Behind => return Ok(false), // already have it
                        Relation::Ahead
                            if self.vault.checkpoint(&head)?.files_digest()
                                == self.vault.checkpoint(checkpoint)?.files_digest() =>
                        {
                            // Same bytes with more history, e.g. the other
                            // device recorded a choice of this device's save.
                            // Adopting it changes no files, so no review.
                            record.head = Some(checkpoint.clone());
                            self.save_bundle(&record)?;
                            return Ok(false);
                        }
                        _ => {}
                    }
                }
            }
            None => self.vault.put_record(
                UNMAPPED,
                bundle.as_str(),
                &UnmappedBundle {
                    bundle_id: bundle.clone(),
                    name_hint: name_hint.to_owned(),
                    from_device: from_device.to_owned(),
                },
            )?,
        }
        for mut existing in self.proposals(bundle)? {
            if !matches!(existing.status, ProposalStatus::Reviewable | ProposalStatus::Parked) {
                continue;
            }
            match relate(&self.vault, &existing.checkpoint, checkpoint) {
                Relation::Ahead => {
                    existing.status = ProposalStatus::Superseded { by: checkpoint.clone() };
                    self.save_proposal(&existing)?;
                }
                Relation::Same | Relation::Behind => return Ok(false),
                _ => {}
            }
        }
        self.save_proposal(&Proposal {
            bundle_id: bundle.clone(),
            checkpoint: checkpoint.clone(),
            from_device: from_device.to_owned(),
            received_at: now_rfc3339(),
            status: ProposalStatus::Reviewable,
        })?;
        Ok(true)
    }

    fn save_proposal(&self, p: &Proposal) -> Result<()> {
        self.vault.put_record(PROPOSALS, &Proposal::key(&p.bundle_id, &p.checkpoint), p)
    }

    /// All proposals ever received for a bundle.
    pub fn proposals(&self, bundle: &BundleId) -> Result<Vec<Proposal>> {
        Ok(self
            .vault
            .list_records::<Proposal>(PROPOSALS)?
            .into_iter()
            .map(|(_, p)| p)
            .filter(|p| &p.bundle_id == bundle)
            .collect())
    }

    pub fn proposal(&self, bundle: &BundleId, cp: &CheckpointId) -> Result<Proposal> {
        self.vault
            .get_record(PROPOSALS, &Proposal::key(bundle, cp))?
            .ok_or_else(|| Error::NotFound("no incoming progress with that id".into()))
    }

    /// Games seen in received data but not set up here yet.
    pub fn unmapped_bundles(&self) -> Result<Vec<UnmappedBundle>> {
        Ok(self.vault.list_records(UNMAPPED)?.into_iter().map(|(_, u)| u).collect())
    }

    /// Proposals waiting for review, across set-up games.
    pub fn incoming(&self) -> Result<Vec<ReviewItem>> {
        let mut out = Vec::new();
        for record in self.bundles()? {
            for proposal in self.proposals(&record.bundle_id)? {
                if proposal.status != ProposalStatus::Reviewable {
                    continue;
                }
                let checkpoint = self.vault.checkpoint(&proposal.checkpoint)?;
                let relation =
                    record.head.as_ref().map(|h| relate(&self.vault, h, &proposal.checkpoint));
                out.push(ReviewItem { proposal, checkpoint, relation });
            }
        }
        Ok(out)
    }

    /// Prepare to use `checkpoint` (incoming progress or a restore).
    ///
    /// Re-verifies the checkpoint's stored bytes. If the playable files have
    /// changes no checkpoint holds, captures them first, so the comparison
    /// and the eventual backup include them. Never writes playable files.
    pub fn review(&self, bundle: &BundleId, checkpoint: &CheckpointId) -> Result<Review> {
        let _gc = self.vault.lock_shared("gc")?;
        let _lock = self.lock_bundle(bundle)?;
        let incoming_checkpoint = self.vault.checkpoint(checkpoint)?;
        if &incoming_checkpoint.bundle_id != bundle {
            return Err(Error::InvalidInput("that checkpoint belongs to another game".into()));
        }
        self.vault.verify_checkpoint_objects(&incoming_checkpoint)?;

        let mut record = self.bundle(bundle)?;
        let location = self.provider.open(&record.location);
        location.check_available()?;
        let live = self.scan(location.as_ref())?;
        let head_digest = match &record.head {
            Some(h) => Some(self.vault.checkpoint(h)?.files_digest()),
            None => None,
        };
        let mut captured_local_changes = None;
        if !live.is_empty() && head_digest.as_ref() != Some(&FilesDigest::of(&live)) {
            let outcome = self.capture_locked(
                bundle,
                CheckpointKind::Capture,
                CaptureMode::QuiescenceHeuristic,
                Some("Saved automatically before reviewing incoming progress".into()),
            )?;
            if let CaptureOutcome::Created(id) = outcome {
                captured_local_changes = Some(id);
            }
            record = self.bundle(bundle)?;
        }
        let baseline_files = if live.is_empty() { None } else { Some(FilesDigest::of(&live)) };
        if record.head.as_ref() == Some(checkpoint) && baseline_files.is_some() {
            return Err(Error::InvalidInput("this save is already in use".into()));
        }
        let relation = record.head.as_ref().map(|h| relate(&self.vault, h, checkpoint));
        let key = crate::records::Proposal::key(bundle, checkpoint);
        let proposal = self.vault.get_record::<Proposal>(PROPOSALS, &key)?.map(|_| key);
        Ok(Review {
            bundle_id: bundle.clone(),
            incoming: checkpoint.clone(),
            incoming_checkpoint,
            baseline_head: record.head,
            baseline_files,
            relation,
            captured_local_changes,
            proposal,
        })
    }

    /// "Keep alongside": stop prompting; the progress stays in history and
    /// can be used later.
    pub fn keep_alongside(&self, bundle: &BundleId, cp: &CheckpointId) -> Result<()> {
        let _lock = self.lock_bundle(bundle)?;
        let mut p = self.proposal(bundle, cp)?;
        p.status = ProposalStatus::Parked;
        self.save_proposal(&p)
    }

    /// Bring parked progress back to Incoming.
    pub fn reopen(&self, bundle: &BundleId, cp: &CheckpointId) -> Result<()> {
        let _lock = self.lock_bundle(bundle)?;
        let mut p = self.proposal(bundle, cp)?;
        if p.status != ProposalStatus::Parked {
            return Err(Error::InvalidInput("only progress kept alongside can be reopened".into()));
        }
        p.status = ProposalStatus::Reviewable;
        self.save_proposal(&p)
    }

    /// "Dismiss": keep this device's save and record that choice in history,
    /// so no device asks about the dismissed progress again. The dismissed
    /// checkpoint itself stays in history. Never writes playable files.
    pub fn dismiss(&self, bundle: &BundleId, cp: &CheckpointId) -> Result<Option<CheckpointId>> {
        let _lock = self.lock_bundle(bundle)?;
        let mut p = self.proposal(bundle, cp)?;
        let mut record = self.bundle(bundle)?;
        let mut choice_id = None;
        if let Some(head) = record.head.clone() {
            let kept = self.vault.checkpoint(&head)?;
            let choice = Checkpoint {
                schema_version: CHECKPOINT_SCHEMA_VERSION,
                bundle_id: bundle.clone(),
                parents: vec![head, cp.clone()],
                kind: CheckpointKind::Choice,
                files: kept.files,
                profile: kept.profile,
                origin_device: self.vault.device_id().clone(),
                origin_device_name: self.vault.device_name().to_owned(),
                captured_at: now_rfc3339(),
                capture_mode: CaptureMode::Automatic,
                note: Some(format!("Kept this device's save over progress from {}", p.from_device)),
            };
            let id = self.vault.put_checkpoint(&choice)?;
            record.head = Some(id.clone());
            self.save_bundle(&record)?;
            choice_id = Some(id);
        }
        p.status = ProposalStatus::Dismissed;
        self.save_proposal(&p)?;
        Ok(choice_id)
    }
}
