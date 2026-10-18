//! Deleting checkpoints, keeping checkpoints for a chosen time, forgetting
//! removed games, and the device settings that go with them.
//!
//! Everything here is local to this device (section 7). Deleting a
//! checkpoint removes only its files: the checkpoint record stays so
//! ancestry keeps deciding how other checkpoints relate, and other devices
//! keep their copies. Nothing here is ever triggered by a missing folder, by
//! another device's state or by a full disk; retention runs only after the
//! user chose a time.

use std::collections::{BTreeMap, HashSet};
use std::time::{Duration, SystemTime};

use doki_core::{BundleId, CheckpointId, CheckpointKind, ContentHash, Error, Result};
use doki_vault::{now_rfc3339, parse_rfc3339};
use serde::{Deserialize, Serialize};

use crate::records::{
    ForgottenGame, PrunedCheckpoint, Settings, UnmappedBundle, FORGOTTEN, PROPOSALS, PRUNED,
    SETTINGS, UNMAPPED, WATCH,
};
use crate::{Engine, LocationProvider};

const SETTINGS_KEY: &str = "device";

/// Retention always keeps at least this many of each game's newest
/// checkpoints, however old, so a game you have not played in a while keeps
/// its recent history.
pub const KEEP_NEWEST: usize = 10;

/// Fields to change with [`Engine::update_settings`]; `None` leaves a field
/// as it is. `keep_days: Some(None)` means keep forever.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SettingsChanges {
    #[serde(default, deserialize_with = "some")]
    pub keep_days: Option<Option<u32>>,
    pub notify_checkpoints: Option<bool>,
    pub notify_changes: Option<bool>,
    pub notify_incoming: Option<bool>,
}

// Distinguishes `"keep_days": null` (keep forever) from a missing field.
fn some<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<Option<u32>>, D::Error> {
    Option::<u32>::deserialize(d).map(Some)
}

/// What deleting or retention removed, or would remove.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Removal {
    pub checkpoints: usize,
    /// Bytes of save files freed. Files another kept checkpoint shares are
    /// not counted, because they stay.
    pub bytes: u64,
}

impl<P: LocationProvider> Engine<P> {
    pub fn settings(&self) -> Result<Settings> {
        Ok(self.vault.get_record(SETTINGS, SETTINGS_KEY)?.unwrap_or_default())
    }

    /// Change settings. A shorter `keep_days` does not remove anything by
    /// itself; call [`Engine::apply_retention`] (after showing
    /// [`Engine::retention_preview`]) to do that.
    pub fn update_settings(&self, changes: SettingsChanges) -> Result<Settings> {
        let mut s = self.settings()?;
        if let Some(days) = changes.keep_days {
            if days == Some(0) {
                return Err(Error::InvalidInput("keep checkpoints for at least one day".into()));
            }
            s.keep_days = days;
        }
        if let Some(v) = changes.notify_checkpoints {
            s.notify_checkpoints = v;
        }
        if let Some(v) = changes.notify_changes {
            s.notify_changes = v;
        }
        if let Some(v) = changes.notify_incoming {
            s.notify_incoming = v;
        }
        self.vault.put_record(SETTINGS, SETTINGS_KEY, &s)?;
        Ok(s)
    }

    /// Whether this device has removed the files of `cp`.
    pub fn is_pruned(&self, cp: &CheckpointId) -> Result<bool> {
        Ok(self.vault.get_record::<PrunedCheckpoint>(PRUNED, cp.as_str())?.is_some())
    }

    pub(crate) fn pruned_ids(&self) -> Result<HashSet<CheckpointId>> {
        self.vault
            .list_records::<PrunedCheckpoint>(PRUNED)?
            .into_iter()
            .map(|(key, _)| CheckpointId::parse(&key))
            .collect()
    }

    pub(crate) fn mark_pruned(&self, bundle: &BundleId, cp: &CheckpointId) -> Result<()> {
        self.vault.put_record(
            PRUNED,
            cp.as_str(),
            &PrunedCheckpoint { bundle_id: bundle.clone(), removed_at: now_rfc3339() },
        )
    }

    /// Checkpoints of `bundle` that must keep their files: the current
    /// save, pinned ones, progress a choice set aside, history tips, any
    /// incoming progress still waiting for a decision, and anything an
    /// install journal refers to.
    fn protected(&self, bundle: &BundleId) -> Result<HashSet<CheckpointId>> {
        let mut keep = HashSet::new();
        if let Ok(record) = self.bundle(bundle) {
            keep.extend(record.head);
            keep.extend(record.pins);
        }
        for e in self.history(bundle)? {
            if e.is_tip || e.set_aside || e.checkpoint.kind == CheckpointKind::Choice {
                keep.insert(e.id);
            }
        }
        for p in self.proposals(bundle)? {
            if matches!(
                p.status,
                crate::records::ProposalStatus::Reviewable | crate::records::ProposalStatus::Parked
            ) {
                keep.insert(p.checkpoint);
            }
        }
        if let Some(j) = self.journal(bundle)? {
            keep.extend(j.target);
            keep.extend(j.backup);
            keep.extend(j.previous_head);
            keep.extend(j.rescued);
        }
        Ok(keep)
    }

    /// Bytes freed if the files of `remove` went, given what is already gone.
    fn bytes_freed(&self, remove: &HashSet<CheckpointId>) -> Result<u64> {
        let pruned = self.pruned_ids()?;
        let mut kept: HashSet<ContentHash> = HashSet::new();
        let mut gone: BTreeMap<ContentHash, u64> = BTreeMap::new();
        for (id, cp) in self.vault.checkpoints()? {
            if pruned.contains(&id) {
                continue;
            }
            for f in cp.files {
                if remove.contains(&id) {
                    gone.insert(f.hash, f.size);
                } else {
                    kept.insert(f.hash);
                }
            }
        }
        Ok(gone.into_iter().filter(|(h, _)| !kept.contains(h)).map(|(_, s)| s).sum())
    }

    /// Remove the files of one checkpoint from this device. Refused for
    /// the current save, pinned checkpoints, incoming progress still waiting
    /// for a decision, and while an install waits for recovery. Other devices
    /// keep their copies. Cannot be undone on this device.
    pub fn delete_checkpoint(&self, bundle: &BundleId, cp: &CheckpointId) -> Result<Removal> {
        let _gc = self.vault.lock("gc")?;
        let _lock = self.lock_bundle(bundle)?;
        let record = self.bundle(bundle)?;
        let checkpoint = self.vault.checkpoint(cp)?;
        if &checkpoint.bundle_id != bundle {
            return Err(Error::InvalidInput("that checkpoint belongs to another game".into()));
        }
        if self.is_pruned(cp)? {
            return Ok(Removal::default());
        }
        if record.head.as_ref() == Some(cp) {
            return Err(Error::InvalidInput(
                "this is the save you are playing; it cannot be deleted".into(),
            ));
        }
        if record.pins.contains(cp) {
            return Err(Error::InvalidInput("unpin this checkpoint before deleting it".into()));
        }
        let waiting = self.proposals(bundle)?.into_iter().any(|p| {
            &p.checkpoint == cp
                && matches!(
                    p.status,
                    crate::records::ProposalStatus::Reviewable
                        | crate::records::ProposalStatus::Parked
                )
        });
        if waiting {
            return Err(Error::InvalidInput(
                "this progress is still in Incoming; dismiss it there instead".into(),
            ));
        }
        let remove: HashSet<CheckpointId> = [cp.clone()].into();
        let bytes = self.bytes_freed(&remove)?;
        self.mark_pruned(bundle, cp)?;
        self.vault.sweep_objects(&HashSet::new(), &self.pruned_ids()?)?;
        Ok(Removal { checkpoints: 1, bytes })
    }

    /// Checkpoints retention would remove if it kept `keep_days` days,
    /// without removing anything.
    pub fn retention_preview(&self, keep_days: u32) -> Result<Removal> {
        let remove: HashSet<CheckpointId> = self
            .retention_candidates(keep_days, SystemTime::now())?
            .into_iter()
            .map(|(cp, _)| cp)
            .collect();
        Ok(Removal { checkpoints: remove.len(), bytes: self.bytes_freed(&remove)? })
    }

    fn retention_candidates(
        &self,
        keep_days: u32,
        now: SystemTime,
    ) -> Result<HashSet<(CheckpointId, BundleId)>> {
        let cutoff = now
            .checked_sub(Duration::from_secs(u64::from(keep_days) * 86_400))
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let pruned = self.pruned_ids()?;
        let mut out = HashSet::new();
        for record in self.bundles()? {
            let bundle = record.bundle_id;
            let keep = self.protected(&bundle)?;
            // History is ordered by ancestry, oldest first; newest first is
            // its reverse. Capture times only decide what counts as old.
            let mut history = self.history(&bundle)?;
            history.reverse();
            for e in history.into_iter().filter(|e| !pruned.contains(&e.id)).skip(KEEP_NEWEST) {
                let old = parse_rfc3339(&e.checkpoint.captured_at).is_some_and(|t| t < cutoff);
                if old && !keep.contains(&e.id) {
                    out.insert((e.id, bundle.clone()));
                }
            }
        }
        Ok(out)
    }

    /// Remove the files of checkpoints older than the chosen retention
    /// time. Does nothing when retention is "forever" (the default).
    ///
    /// Always keeps, per game: the [`KEEP_NEWEST`] newest checkpoints, the
    /// current save, pinned checkpoints, history tips, progress a choice set
    /// aside, incoming progress still waiting for a decision, and anything an
    /// install journal refers to. Checkpoint records stay for ancestry.
    pub fn apply_retention(&self) -> Result<Removal> {
        self.apply_retention_at(SystemTime::now())
    }

    pub(crate) fn apply_retention_at(&self, now: SystemTime) -> Result<Removal> {
        let Some(days) = self.settings()?.keep_days else {
            return Ok(Removal::default());
        };
        let _gc = self.vault.lock("gc")?;
        let mut locks = Vec::new();
        for b in self.bundles()? {
            locks.push(self.lock_bundle(&b.bundle_id)?);
        }
        let candidates = self.retention_candidates(days, now)?;
        if candidates.is_empty() {
            return Ok(Removal::default());
        }
        let ids: HashSet<CheckpointId> = candidates.iter().map(|(c, _)| c.clone()).collect();
        let bytes = self.bytes_freed(&ids)?;
        for (cp, bundle) in &candidates {
            self.mark_pruned(bundle, cp)?;
        }
        self.vault.sweep_objects(&HashSet::new(), &self.pruned_ids()?)?;
        Ok(Removal { checkpoints: candidates.len(), bytes })
    }

    /// Delete a removed game from this device: its entry under "set up
    /// again", its checkpoints and their files, and any progress received
    /// for it. Only for games not set up here (remove it first). Other
    /// devices keep their copies; from now on Doki ignores what they send for
    /// it, until [`Engine::unforget_game`].
    pub fn forget_game(&self, bundle: &BundleId) -> Result<Removal> {
        let _gc = self.vault.lock("gc")?;
        let _lock = self.vault.lock(&format!("bundle-{bundle}"))?;
        if self.bundle(bundle).is_ok() {
            return Err(Error::InvalidInput(
                "this game is set up here; remove it from this device first".into(),
            ));
        }
        let entry: Option<UnmappedBundle> = self.vault.get_record(UNMAPPED, bundle.as_str())?;
        let ids: HashSet<CheckpointId> = self
            .vault
            .checkpoints()?
            .into_iter()
            .filter(|(_, cp)| &cp.bundle_id == bundle)
            .map(|(id, _)| id)
            .collect();
        if entry.is_none() && ids.is_empty() {
            return Err(Error::NotFound("no removed game with that id".into()));
        }
        let bytes = self.bytes_freed(&ids)?;
        self.vault.put_record(
            FORGOTTEN,
            bundle.as_str(),
            &ForgottenGame {
                bundle_id: bundle.clone(),
                name: entry.map(|e| e.name_hint).unwrap_or_default(),
                forgotten_at: now_rfc3339(),
            },
        )?;
        for p in self.proposals(bundle)? {
            self.vault
                .delete_record(PROPOSALS, &crate::records::Proposal::key(bundle, &p.checkpoint))?;
        }
        for id in &ids {
            self.vault.delete_record(PRUNED, id.as_str())?;
            self.vault.delete_checkpoint(id)?;
        }
        self.vault.delete_record(WATCH, bundle.as_str())?;
        self.vault.delete_record(UNMAPPED, bundle.as_str())?;
        self.vault.sweep_objects(&HashSet::new(), &self.pruned_ids()?)?;
        Ok(Removal { checkpoints: ids.len(), bytes })
    }

    pub fn is_forgotten(&self, bundle: &BundleId) -> Result<bool> {
        Ok(self.vault.get_record::<ForgottenGame>(FORGOTTEN, bundle.as_str())?.is_some())
    }

    /// Games deleted with [`Engine::forget_game`].
    pub fn forgotten_games(&self) -> Result<Vec<ForgottenGame>> {
        Ok(self.vault.list_records(FORGOTTEN)?.into_iter().map(|(_, g)| g).collect())
    }

    /// Stop ignoring a deleted game. It comes back with the next sync if
    /// another device still has it.
    pub fn unforget_game(&self, bundle: &BundleId) -> Result<()> {
        self.vault.delete_record(FORGOTTEN, bundle.as_str())
    }
}
