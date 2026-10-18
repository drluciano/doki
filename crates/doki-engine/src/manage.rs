//! Changing and removing set-up games, and automatic checkpoints.

use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use doki_core::{BundleId, CaptureMode, CheckpointId, Error, FilesDigest, Result};
use serde::{Deserialize, Serialize};

use crate::records::{UnmappedBundle, BUNDLES, UNMAPPED, WATCH};
use crate::{is_within, normalize_location, CaptureOutcome, Engine, LocationProvider};

/// Fields to change with [`Engine::update_bundle`]; `None` leaves a field as
/// it is.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct BundleChanges {
    pub name: Option<String>,
    pub location: Option<String>,
    pub auto_capture: Option<bool>,
}

/// What [`Engine::auto_check`] did for one game.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "outcome")]
pub enum AutoOutcome {
    /// The folder matches the current checkpoint.
    Unchanged,
    /// A change was just noticed, after the folder last matched the
    /// current checkpoint. Reported once per change, for notifications.
    Detected,
    /// The folder changed and is waiting to stay still for the settle time.
    Waiting,
    /// A checkpoint was taken.
    Created { checkpoint: CheckpointId },
    /// Not checked this time; `code` is an error identifier (section 11).
    Skipped { code: String, message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AutoCheck {
    pub bundle_id: BundleId,
    pub name: String,
    #[serde(flatten)]
    pub outcome: AutoOutcome,
}

/// The last different folder state seen, and since when. Key: bundle id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Watch {
    digest: FilesDigest,
    since_secs: u64,
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

impl<P: LocationProvider> Engine<P> {
    /// Check a save folder for a game and return it in stored form.
    /// `except` is the game being changed, which may keep its own folder.
    pub(crate) fn check_location(
        &self,
        location: &str,
        except: Option<&BundleId>,
    ) -> Result<String> {
        self.check_folder(location, except, None)
    }

    /// Check a folder for a game or a whole system (`except_system`, when a
    /// system is being changed) and return it in stored form. A folder can
    /// belong to one game or one system, and never contain Doki's vault.
    pub(crate) fn check_folder(
        &self,
        location: &str,
        except: Option<&BundleId>,
        except_system: Option<&BundleId>,
    ) -> Result<String> {
        self.provider.open(location).check_available()?;
        let location = normalize_location(location);
        if is_within(self.vault.root(), Path::new(&location))
            || is_within(Path::new(&location), self.vault.root())
        {
            return Err(Error::InvalidInput(
                "the save folder and Doki's vault must not contain each other".into(),
            ));
        }
        for existing in self.bundles()? {
            if existing.member.is_none()
                && existing.location == location
                && Some(&existing.bundle_id) != except
            {
                return Err(Error::InvalidInput(format!(
                    "this folder is already set up for {}",
                    existing.name
                )));
            }
        }
        for system in self.systems()? {
            if system.location == location && Some(&system.system_id) != except_system {
                return Err(Error::InvalidInput(format!(
                    "this folder is already set up as the system {}",
                    system.name
                )));
            }
        }
        Ok(location)
    }

    /// Rename a game, point it at another save folder, or turn automatic
    /// checkpoints on or off. Never reads or writes save files: after a
    /// folder change the next capture records whatever the new folder holds,
    /// as a child of the current checkpoint. Refused while an interrupted
    /// install waits for recovery.
    pub fn update_bundle(
        &self,
        bundle: &BundleId,
        changes: BundleChanges,
    ) -> Result<crate::BundleRecord> {
        let _lock = self.lock_bundle(bundle)?;
        let mut record = self.bundle(bundle)?;
        if let Some(name) = changes.name {
            if name.trim().is_empty() {
                return Err(Error::InvalidInput("a game name is required".into()));
            }
            record.name = name.trim().to_owned();
        }
        if let Some(location) = changes.location {
            if record.member.is_some() {
                return Err(Error::InvalidInput(
                    "this game is part of a system; change the system's folder instead".into(),
                ));
            }
            let location = self.check_location(&location, Some(bundle))?;
            if location != record.location {
                record.location = location;
                // Folder state seen in the old folder means nothing here.
                self.vault.delete_record(WATCH, bundle.as_str())?;
            }
        }
        if let Some(auto) = changes.auto_capture {
            record.auto_capture = auto;
        }
        self.save_bundle(&record)?;
        Ok(record)
    }

    /// Remove a game from this device only. The save folder is not touched
    /// and every checkpoint stays in the vault; other devices are not told.
    /// The game is listed for setup again, so setting it up (with any
    /// folder) brings its history back. Refused while an interrupted install
    /// waits for recovery.
    pub fn remove_bundle(&self, bundle: &BundleId) -> Result<()> {
        let _lock = self.lock_bundle(bundle)?;
        let record = self.bundle(bundle)?;
        self.drop_offers(bundle)?;
        self.vault.delete_record(WATCH, bundle.as_str())?;
        if let Some(member) = &record.member {
            // Part of a system: stop tracking it so the next scan does not
            // add it straight back. "Track again" brings it and its history.
            self.ignore_member(member)?;
            return self.vault.delete_record(BUNDLES, bundle.as_str());
        }
        self.vault.put_record(
            UNMAPPED,
            bundle.as_str(),
            &UnmappedBundle {
                bundle_id: bundle.clone(),
                name_hint: record.name.clone(),
                from_device: self.vault.device_name().to_owned(),
                has_progress: record.head.is_some(),
                member: None,
            },
        )?;
        self.vault.delete_record(BUNDLES, bundle.as_str())
    }

    /// Look at every game with automatic checkpoints on and capture the ones
    /// whose folder changed and then stayed the same for `settle`. First
    /// adds any new games in whole-system folders ([`Engine::scan_systems`]).
    ///
    /// Safe to call often (the app calls it every few seconds). A change is
    /// only captured after two looks at least `settle` apart see the same
    /// bytes, and capture itself re-reads the folder and refuses if it moved
    /// (section 4), so a capture never lands mid-write. Never writes save
    /// files. Games that cannot be read are skipped and reported, not
    /// errors. After a checkpoint is taken, applies the retention time the
    /// user chose, if any ([`Engine::apply_retention`]).
    pub fn auto_check(&self, settle: Duration) -> Result<Vec<AutoCheck>> {
        self.scan_systems()?;
        let mut out = Vec::new();
        for record in self.bundles()? {
            if !record.auto_capture {
                continue;
            }
            let outcome = match self.auto_check_one(&record, settle) {
                Ok(o) => o,
                Err(e) => {
                    AutoOutcome::Skipped { code: e.code().to_owned(), message: e.to_string() }
                }
            };
            out.push(AutoCheck { bundle_id: record.bundle_id, name: record.name, outcome });
        }
        if out.iter().any(|c| matches!(c.outcome, AutoOutcome::Created { .. })) {
            // A new checkpoint may push an old one past the retention time.
            // A failure here (another operation running, a game waiting for
            // recovery) only delays cleanup to a later check.
            let _ = self.apply_retention();
        }
        Ok(out)
    }

    fn auto_check_one(
        &self,
        record: &crate::BundleRecord,
        settle: Duration,
    ) -> Result<AutoOutcome> {
        let bundle = &record.bundle_id;
        if self.journal(bundle)?.is_some() {
            return Err(Error::RecoveryRequired);
        }
        let location = self.open_bundle(record);
        location.check_available()?;
        let files = self.scan(location.as_ref())?;
        let digest = FilesDigest::of(&files);
        let at_head = match &record.head {
            Some(head) => self.vault.checkpoint(head)?.files_digest() == digest,
            None => files.is_empty(),
        };
        if at_head {
            self.vault.delete_record(WATCH, bundle.as_str())?;
            return Ok(AutoOutcome::Unchanged);
        }
        let now = now_secs();
        let seen: Option<Watch> = self.vault.get_record(WATCH, bundle.as_str())?;
        match seen {
            Some(w)
                if w.digest == digest && now.saturating_sub(w.since_secs) >= settle.as_secs() => {}
            Some(w) if w.digest == digest => return Ok(AutoOutcome::Waiting),
            seen => {
                self.vault.put_record(
                    WATCH,
                    bundle.as_str(),
                    &Watch { digest, since_secs: now },
                )?;
                return Ok(if seen.is_none() {
                    AutoOutcome::Detected
                } else {
                    AutoOutcome::Waiting
                });
            }
        }
        match self.capture(bundle, CaptureMode::QuiescenceHeuristic, None) {
            Ok(CaptureOutcome::Created(id)) => {
                self.vault.delete_record(WATCH, bundle.as_str())?;
                Ok(AutoOutcome::Created { checkpoint: id })
            }
            Ok(CaptureOutcome::Unchanged(_)) => {
                self.vault.delete_record(WATCH, bundle.as_str())?;
                Ok(AutoOutcome::Unchanged)
            }
            Err(Error::SourceChanging) => {
                // Moved while copying: start waiting again from now.
                self.vault.delete_record(WATCH, bundle.as_str())?;
                Ok(AutoOutcome::Waiting)
            }
            Err(e) => Err(e),
        }
    }
}
