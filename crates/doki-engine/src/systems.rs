//! Whole-system save folders.
//!
//! A system is one emulator save folder whose games are found and tracked
//! one by one, so the user does not set up each game. Every subfolder, and
//! every group of loose files sharing a name, becomes a game of its own
//! ([`doki_platform::member::discover`]) with ordinary checkpoints, history,
//! review and recovery. Games that appear later are added on the next scan,
//! which [`Engine::auto_check`] runs every time.
//!
//! Linking: setting up a system that another device announced reuses its
//! id. A game's id is derived from the system id and its subfolder or file
//! name ([`MemberRef::bundle_id`]), so the same game in a linked system is
//! the same game on both devices. Linking is the user's explicit choice;
//! Doki never matches games by title. Received progress still waits in
//! Incoming like any other.
//!
//! Like games, systems are removed from this device only. Removing a game
//! from a system stops tracking it (it is not added back by the next scan)
//! and keeps its history.

use std::collections::HashSet;

use doki_core::{BundleId, Error, Result};
use doki_platform::member::discover;
use doki_vault::{now_rfc3339, random_hex};
use serde::Deserialize;

use crate::records::{
    MemberRef, ProposalStatus, RemoteSystem, SystemRecord, UnmappedBundle, BUNDLES, PROPOSALS,
    REMOTE_SYSTEMS, SYSTEMS, UNMAPPED, WATCH,
};
use crate::{generic_folder_profile, BundleRecord, Engine, LocationProvider};

/// Fields to change with [`Engine::update_system`]; `None` leaves a field.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SystemChanges {
    pub name: Option<String>,
    pub location: Option<String>,
    /// Applies to games found from now on and to every current game.
    pub auto_capture: Option<bool>,
}

impl<P: LocationProvider> Engine<P> {
    /// Set up a whole-system save folder and add the games already in it.
    ///
    /// Pass `link` to join a system announced by another device (see
    /// [`Engine::remote_systems`]). Never reads save files beyond listing the
    /// folder's top level.
    pub fn add_system(
        &self,
        name: &str,
        location: &str,
        link: Option<BundleId>,
    ) -> Result<SystemRecord> {
        if name.trim().is_empty() {
            return Err(Error::InvalidInput("a name for the system is required".into()));
        }
        let location = self.check_folder(location, None, None)?;
        if let Some(id) = &link {
            if let Some(existing) = self.vault.get_record::<SystemRecord>(SYSTEMS, id.as_str())? {
                return Err(Error::InvalidInput(format!(
                    "this system is already set up as {}",
                    existing.name
                )));
            }
        }
        let record = SystemRecord {
            system_id: link.unwrap_or(BundleId::parse(&random_hex(8))?),
            name: name.trim().to_owned(),
            location,
            created_at: now_rfc3339(),
            auto_capture: true,
            ignored: Default::default(),
        };
        {
            let _lock = self.vault.lock("systems")?;
            self.vault.put_record(SYSTEMS, record.system_id.as_str(), &record)?;
        }
        self.vault.delete_record(REMOTE_SYSTEMS, record.system_id.as_str())?;
        self.scan_system(&record)?;
        Ok(record)
    }

    pub fn systems(&self) -> Result<Vec<SystemRecord>> {
        Ok(self.vault.list_records(SYSTEMS)?.into_iter().map(|(_, s)| s).collect())
    }

    pub fn system(&self, id: &BundleId) -> Result<SystemRecord> {
        self.vault
            .get_record(SYSTEMS, id.as_str())?
            .ok_or_else(|| Error::NotFound(format!("system {}", id.short())))
    }

    /// Find a system by id prefix or case-insensitive name.
    pub fn find_system(&self, query: &str) -> Result<SystemRecord> {
        let q = query.to_lowercase();
        let mut matches: Vec<_> = self
            .systems()?
            .into_iter()
            .filter(|s| s.system_id.as_str().starts_with(&q) || s.name.to_lowercase() == q)
            .collect();
        match matches.len() {
            1 => Ok(matches.remove(0)),
            0 => Err(Error::NotFound(format!("no system matches {query:?}"))),
            _ => Err(Error::InvalidInput(format!("{query:?} matches several systems"))),
        }
    }

    /// Games of a system set up on this device.
    pub fn system_games(&self, id: &BundleId) -> Result<Vec<BundleRecord>> {
        Ok(self
            .bundles()?
            .into_iter()
            .filter(|b| b.member.as_ref().is_some_and(|m| &m.system_id == id))
            .collect())
    }

    /// Add new games in every system folder. Systems whose folder cannot be
    /// read are skipped; their games already show why. Returns the games
    /// added.
    pub fn scan_systems(&self) -> Result<Vec<BundleRecord>> {
        let mut added = Vec::new();
        for system in self.systems()? {
            if let Ok(mut found) = self.scan_system(&system) {
                added.append(&mut found);
            }
        }
        Ok(added)
    }

    /// Add the games in one system folder that are not set up yet, and the
    /// games of this system that other devices have announced.
    ///
    /// Runs under the `systems` lock so two scans (the app and Android's
    /// background service) never both create a game and overwrite each
    /// other's record. Never reads save files.
    pub(crate) fn scan_system(&self, system: &SystemRecord) -> Result<Vec<BundleRecord>> {
        let entries = self.provider.open(&system.location).entries()?;
        let mut members: Vec<MemberRef> = discover(&entries)
            .iter()
            .map(|m| MemberRef::from_member(&system.system_id, m))
            .collect();
        for u in self.unmapped_bundles_raw()? {
            if let Some(m) = u.member {
                if m.system_id == system.system_id && m.bundle_id() == u.bundle_id {
                    members.push(m);
                }
            }
        }
        let _lock = self.vault.lock("systems")?;
        // Re-read under the lock: the user may have changed it meanwhile.
        let system = self.system(&system.system_id)?;
        let mut added = Vec::new();
        let mut seen = HashSet::new();
        for member in members {
            let id = member.bundle_id();
            if system.ignored.contains(&member.key()) || !seen.insert(id.clone()) {
                continue;
            }
            if self.vault.get_record::<BundleRecord>(BUNDLES, id.as_str())?.is_some() {
                continue;
            }
            added.push(self.add_member(&system, member)?);
        }
        Ok(added)
    }

    /// Create a game record for a member. Caller holds the `systems` lock
    /// and has checked the game is not set up.
    fn add_member(&self, system: &SystemRecord, member: MemberRef) -> Result<BundleRecord> {
        let unmapped: Option<UnmappedBundle> =
            self.vault.get_record(UNMAPPED, member.bundle_id().as_str())?;
        let record = BundleRecord {
            bundle_id: member.bundle_id(),
            // A name chosen on another device beats the folder name.
            name: unmapped.map(|u| u.name_hint).unwrap_or_else(|| member.name.clone()),
            location: system.location.clone(),
            profile: generic_folder_profile(),
            head: None,
            pins: Default::default(),
            created_at: now_rfc3339(),
            auto_capture: system.auto_capture,
            member: Some(member),
        };
        self.save_bundle(&record)?;
        self.vault.delete_record(UNMAPPED, record.bundle_id.as_str())?;
        Ok(record)
    }

    /// Rename a system, point it at another folder, or turn automatic
    /// checkpoints on or off for all its games. Never reads or writes save
    /// files. A folder change moves every game of the system with it and is
    /// refused while any of them waits for recovery.
    pub fn update_system(&self, id: &BundleId, changes: SystemChanges) -> Result<SystemRecord> {
        let mut system = self.system(id)?;
        if let Some(name) = changes.name {
            if name.trim().is_empty() {
                return Err(Error::InvalidInput("a name for the system is required".into()));
            }
            system.name = name.trim().to_owned();
        }
        let games = self.system_games(id)?;
        let mut locks = Vec::new();
        for g in &games {
            locks.push(self.lock_bundle(&g.bundle_id)?);
        }
        let location = match changes.location {
            Some(l) => Some(self.check_folder(&l, None, Some(id))?),
            None => None,
        };
        if let Some(auto) = changes.auto_capture {
            system.auto_capture = auto;
        }
        for mut g in games {
            if let Some(l) = &location {
                if &g.location != l {
                    g.location = l.clone();
                    self.vault.delete_record(WATCH, g.bundle_id.as_str())?;
                }
            }
            if let Some(auto) = changes.auto_capture {
                g.auto_capture = auto;
            }
            self.save_bundle(&g)?;
        }
        if let Some(l) = location {
            system.location = l;
        }
        let _lock = self.vault.lock("systems")?;
        let ignored = self.system(id)?.ignored; // may have changed meanwhile
        system.ignored = ignored;
        self.vault.put_record(SYSTEMS, id.as_str(), &system)?;
        Ok(system)
    }

    /// Remove a system and all its games from this device only. Save files
    /// and checkpoints stay; the system is listed for setup again, and
    /// setting it up brings its games and their history back.
    pub fn remove_system(&self, id: &BundleId) -> Result<()> {
        let system = self.system(id)?;
        let games = self.system_games(id)?;
        let mut locks = Vec::new();
        for g in &games {
            locks.push(self.lock_bundle(&g.bundle_id)?);
        }
        let _lock = self.vault.lock("systems")?;
        for g in games {
            self.drop_offers(&g.bundle_id)?;
            self.vault.delete_record(WATCH, g.bundle_id.as_str())?;
            self.vault.put_record(
                UNMAPPED,
                g.bundle_id.as_str(),
                &UnmappedBundle {
                    bundle_id: g.bundle_id.clone(),
                    name_hint: g.name.clone(),
                    from_device: self.vault.device_name().to_owned(),
                    has_progress: g.head.is_some(),
                    member: g.member.clone(),
                },
            )?;
            self.vault.delete_record(BUNDLES, g.bundle_id.as_str())?;
        }
        self.vault.put_record(
            REMOTE_SYSTEMS,
            id.as_str(),
            &RemoteSystem {
                system_id: id.clone(),
                name: system.name,
                from_device: self.vault.device_name().to_owned(),
            },
        )?;
        self.vault.delete_record(SYSTEMS, id.as_str())
    }

    /// Delete offers still waiting for a game being removed; they come back
    /// with the next sync if still relevant.
    pub(crate) fn drop_offers(&self, bundle: &BundleId) -> Result<()> {
        for proposal in self.proposals(bundle)? {
            if matches!(proposal.status, ProposalStatus::Reviewable | ProposalStatus::Parked) {
                self.vault.delete_record(
                    PROPOSALS,
                    &crate::records::Proposal::key(bundle, &proposal.checkpoint),
                )?;
            }
        }
        Ok(())
    }

    /// Stop tracking a game of a system: a later scan does not add it back.
    /// Caller holds the game's lock and removes its record.
    pub(crate) fn ignore_member(&self, member: &MemberRef) -> Result<()> {
        let _lock = self.vault.lock("systems")?;
        if let Some(mut system) =
            self.vault.get_record::<SystemRecord>(SYSTEMS, member.system_id.as_str())?
        {
            system.ignored.insert(member.key());
            self.vault.put_record(SYSTEMS, system.system_id.as_str(), &system)?;
        }
        Ok(())
    }

    /// Track a game the user removed from a system again, with its history.
    pub fn track_again(&self, id: &BundleId, key: &str) -> Result<Vec<BundleRecord>> {
        {
            let _lock = self.vault.lock("systems")?;
            let mut system = self.system(id)?;
            if !system.ignored.remove(key) {
                return Err(Error::NotFound(format!("{key} is not hidden in {}", system.name)));
            }
            self.vault.put_record(SYSTEMS, id.as_str(), &system)?;
        }
        self.scan_system(&self.system(id)?)
    }

    /// Record that another device has this system, so it can be set up
    /// here with one tap. Never touches save files.
    pub fn note_remote_system(&self, id: &BundleId, name: &str, from_device: &str) -> Result<()> {
        if self.vault.get_record::<SystemRecord>(SYSTEMS, id.as_str())?.is_some() {
            return Ok(());
        }
        self.vault.put_record(
            REMOTE_SYSTEMS,
            id.as_str(),
            &RemoteSystem {
                system_id: id.clone(),
                name: name.to_owned(),
                from_device: from_device.to_owned(),
            },
        )
    }

    /// Systems on other devices that are not set up here.
    pub fn remote_systems(&self) -> Result<Vec<RemoteSystem>> {
        Ok(self.vault.list_records(REMOTE_SYSTEMS)?.into_iter().map(|(_, s)| s).collect())
    }

    /// Record that `bundle` is a game of a system on another device. When
    /// that system is set up here, the game is set up as part of it (its
    /// progress then waits in Incoming); otherwise it is remembered for when
    /// the system is. Ignores a claim whose id does not match the member,
    /// so another device cannot attach a game to the wrong system.
    pub fn note_remote_member(
        &self,
        bundle: &BundleId,
        member: &MemberRef,
        name_hint: &str,
        from_device: &str,
    ) -> Result<()> {
        if &member.bundle_id() != bundle
            || self.vault.get_record::<BundleRecord>(BUNDLES, bundle.as_str())?.is_some()
            || self.is_forgotten(bundle)?
        {
            return Ok(());
        }
        let mut unmapped: UnmappedBundle =
            self.vault.get_record(UNMAPPED, bundle.as_str())?.unwrap_or(UnmappedBundle {
                bundle_id: bundle.clone(),
                name_hint: name_hint.to_owned(),
                from_device: from_device.to_owned(),
                has_progress: false,
                member: None,
            });
        unmapped.member = Some(member.clone());
        self.vault.put_record(UNMAPPED, bundle.as_str(), &unmapped)?;
        let _lock = self.vault.lock("systems")?;
        if let Some(system) =
            self.vault.get_record::<SystemRecord>(SYSTEMS, member.system_id.as_str())?
        {
            let set_up = self.vault.get_record::<BundleRecord>(BUNDLES, bundle.as_str())?.is_some();
            if !set_up && !system.ignored.contains(&member.key()) {
                self.add_member(&system, member.clone())?;
            }
        }
        Ok(())
    }

    /// The system membership already known for a game not set up here.
    pub(crate) fn unmapped_member(&self, bundle: &BundleId) -> Result<Option<MemberRef>> {
        Ok(self
            .vault
            .get_record::<UnmappedBundle>(UNMAPPED, bundle.as_str())?
            .and_then(|u| u.member))
    }

    pub(crate) fn unmapped_bundles_raw(&self) -> Result<Vec<UnmappedBundle>> {
        Ok(self.vault.list_records(UNMAPPED)?.into_iter().map(|(_, u)| u).collect())
    }

    /// Ids of games the user stopped tracking in a system here.
    pub(crate) fn ignored_members(&self) -> Result<HashSet<BundleId>> {
        let mut out = HashSet::new();
        for system in self.systems()? {
            for key in &system.ignored {
                if let Some((kind, name)) = key.split_once(':') {
                    let kind = match kind {
                        "folder" => crate::MemberKind::Folder,
                        _ => crate::MemberKind::Files,
                    };
                    let m =
                        MemberRef { system_id: system.system_id.clone(), kind, name: name.into() };
                    out.insert(m.bundle_id());
                }
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::time::Duration;

    use doki_core::CaptureMode;
    use tempfile::TempDir;

    use super::*;
    use crate::BundleChanges;

    struct Device {
        _vault: TempDir,
        folder: TempDir,
        engine: Engine,
    }

    fn device(name: &str) -> Device {
        let vault = tempfile::tempdir().unwrap();
        doki_vault::Vault::init(vault.path(), name).unwrap();
        let engine = Engine::open(vault.path()).unwrap();
        Device { _vault: vault, folder: tempfile::tempdir().unwrap(), engine }
    }

    fn write(dir: &Path, rel: &str, bytes: &[u8]) {
        let p = dir.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, bytes).unwrap();
    }

    impl Device {
        fn loc(&self) -> &str {
            self.folder.path().to_str().unwrap()
        }

        fn game(&self, name: &str) -> BundleRecord {
            self.engine.find_bundle(name).unwrap()
        }

        fn names(&self) -> Vec<String> {
            let mut v: Vec<_> =
                self.engine.bundles().unwrap().into_iter().map(|b| b.name).collect();
            v.sort();
            v
        }

        fn capture(&self, name: &str) {
            let id = self.game(name).bundle_id;
            self.engine.capture(&id, CaptureMode::EmulatorStoppedConfirmed, None).unwrap();
        }
    }

    #[test]
    fn every_game_in_the_folder_is_tracked_including_new_ones() {
        let pc = device("PC");
        write(pc.folder.path(), "Zelda/user/save.bin", b"z1");
        write(pc.folder.path(), "Emerald.sav", b"e1");
        write(pc.folder.path(), "Emerald.ss1", b"state");
        let system = pc.engine.add_system("mGBA", pc.loc(), None).unwrap();
        assert_eq!(pc.names(), vec!["Emerald", "Zelda"]);

        pc.capture("Zelda");
        let head = pc.game("Zelda").head.unwrap();
        let paths: Vec<_> = pc
            .engine
            .vault()
            .checkpoint(&head)
            .unwrap()
            .files
            .iter()
            .map(|f| f.path.to_string())
            .collect();
        assert_eq!(paths, vec!["user/save.bin"], "paths are relative to the game");
        pc.capture("Emerald");
        let head = pc.game("Emerald").head.unwrap();
        assert_eq!(pc.engine.vault().checkpoint(&head).unwrap().files.len(), 2);

        // A game played for the first time shows up on the next check.
        write(pc.folder.path(), "Metroid/save.bin", b"m1");
        pc.engine.auto_check(Duration::from_secs(30)).unwrap();
        assert_eq!(pc.names(), vec!["Emerald", "Metroid", "Zelda"]);
        assert_eq!(pc.engine.system_games(&system.system_id).unwrap().len(), 3);
    }

    #[test]
    fn removing_a_game_stops_tracking_it_until_asked() {
        let pc = device("PC");
        write(pc.folder.path(), "Zelda/save.bin", b"z1");
        let system = pc.engine.add_system("Eden", pc.loc(), None).unwrap();
        pc.capture("Zelda");
        let zelda = pc.game("Zelda");

        pc.engine.remove_bundle(&zelda.bundle_id).unwrap();
        pc.engine.scan_systems().unwrap();
        assert!(pc.names().is_empty(), "not added back by a scan");
        assert!(pc.engine.unmapped_bundles().unwrap().is_empty());
        assert_eq!(fs::read(pc.folder.path().join("Zelda/save.bin")).unwrap(), b"z1");

        let key = pc.engine.system(&system.system_id).unwrap().ignored.into_iter().next().unwrap();
        assert_eq!(key, "folder:Zelda");
        pc.engine.track_again(&system.system_id, &key).unwrap();
        assert_eq!(pc.names(), vec!["Zelda"]);
        assert_eq!(pc.engine.history(&zelda.bundle_id).unwrap().len(), 1, "history kept");
    }

    #[test]
    fn linked_systems_share_games_and_progress_waits_for_review() {
        let pc = device("PC");
        write(pc.folder.path(), "Zelda/save.bin", b"pc progress");
        let system = pc.engine.add_system("Eden", pc.loc(), None).unwrap();
        pc.capture("Zelda");

        let handheld = device("Handheld");
        write(handheld.folder.path(), "Zelda/save.bin", b"handheld progress");
        write(handheld.folder.path(), "Metroid/save.bin", b"m");

        let dir = tempfile::tempdir().unwrap();
        let pkg = dir.path().join("p.dokipkg");
        pc.engine.export_package(&[pc.game("Zelda").bundle_id], &pkg).unwrap();
        handheld.engine.import_package(&pkg).unwrap();
        // Not set up here yet: waits, nothing touched.
        assert_eq!(handheld.engine.unmapped_bundles().unwrap().len(), 1);

        handheld.engine.note_remote_system(&system.system_id, "Eden", "PC").unwrap();
        assert_eq!(handheld.engine.remote_systems().unwrap().len(), 1);
        handheld.engine.add_system("Eden", handheld.loc(), Some(system.system_id.clone())).unwrap();
        assert!(handheld.engine.remote_systems().unwrap().is_empty());
        assert!(handheld.engine.unmapped_bundles().unwrap().is_empty());
        assert_eq!(handheld.names(), vec!["Metroid", "Zelda"]);
        assert_eq!(handheld.game("Zelda").bundle_id, pc.game("Zelda").bundle_id, "same game");

        // The PC's progress is offered, never applied.
        let incoming = handheld.engine.incoming().unwrap();
        assert_eq!(incoming.len(), 1);
        let save = handheld.folder.path().join("Zelda/save.bin");
        assert_eq!(fs::read(&save).unwrap(), b"handheld progress");

        let zelda = handheld.game("Zelda").bundle_id;
        let review = handheld.engine.review(&zelda, &incoming[0].proposal.checkpoint).unwrap();
        handheld.engine.use_checkpoint(&review).unwrap();
        assert_eq!(fs::read(&save).unwrap(), b"pc progress");
        assert_eq!(fs::read(handheld.folder.path().join("Metroid/save.bin")).unwrap(), b"m");
        // The handheld's own save was captured before being replaced.
        assert!(handheld.engine.history(&zelda).unwrap().len() >= 2);
    }

    #[test]
    fn a_game_only_played_elsewhere_installs_into_a_new_subfolder() {
        let pc = device("PC");
        write(pc.folder.path(), "Zelda/save.bin", b"pc");
        let system = pc.engine.add_system("Eden", pc.loc(), None).unwrap();
        pc.capture("Zelda");

        let handheld = device("Handheld");
        handheld.engine.add_system("Eden", handheld.loc(), Some(system.system_id.clone())).unwrap();
        let zelda = pc.game("Zelda");
        handheld
            .engine
            .note_remote_member(&zelda.bundle_id, zelda.member.as_ref().unwrap(), "Zelda", "PC")
            .unwrap();
        assert_eq!(handheld.names(), vec!["Zelda"]);

        let dir = tempfile::tempdir().unwrap();
        let pkg = dir.path().join("p.dokipkg");
        pc.engine.export_package(std::slice::from_ref(&zelda.bundle_id), &pkg).unwrap();
        let summary = handheld.engine.import_package(&pkg).unwrap();
        assert_eq!(summary.new_proposals.len(), 1);
        let review = handheld.engine.review(&zelda.bundle_id, &summary.new_proposals[0].1).unwrap();
        handheld.engine.use_checkpoint(&review).unwrap();
        assert_eq!(fs::read(handheld.folder.path().join("Zelda/save.bin")).unwrap(), b"pc");
    }

    #[test]
    fn a_wrong_member_claim_is_ignored() {
        let pc = device("PC");
        let system = pc.engine.add_system("Eden", pc.loc(), None).unwrap();
        let member = MemberRef {
            system_id: system.system_id,
            kind: crate::MemberKind::Folder,
            name: "Zelda".into(),
        };
        let other = BundleId::parse("00112233aabbccdd").unwrap();
        pc.engine.note_remote_member(&other, &member, "Zelda", "Mallory").unwrap();
        assert!(pc.names().is_empty());
        assert!(pc.engine.unmapped_bundles().unwrap().is_empty());
    }

    #[test]
    fn folders_belong_to_one_game_or_one_system() {
        let pc = device("PC");
        pc.engine.add_system("Eden", pc.loc(), None).unwrap();
        assert_eq!(
            pc.engine.add_bundle("Zelda", pc.loc(), None).unwrap_err().code(),
            "InvalidInput"
        );
        assert_eq!(
            pc.engine.add_system("Again", pc.loc(), None).unwrap_err().code(),
            "InvalidInput"
        );
        let game = tempfile::tempdir().unwrap();
        let g = game.path().to_str().unwrap();
        pc.engine.add_bundle("Zelda", g, None).unwrap();
        assert_eq!(pc.engine.add_system("Other", g, None).unwrap_err().code(), "InvalidInput");
    }

    #[test]
    fn moving_or_removing_a_system_moves_or_removes_its_games() {
        let pc = device("PC");
        write(pc.folder.path(), "Zelda/save.bin", b"z");
        let system = pc.engine.add_system("Eden", pc.loc(), None).unwrap();
        pc.capture("Zelda");

        let moved = tempfile::tempdir().unwrap();
        write(moved.path(), "Zelda/save.bin", b"z");
        let changes = SystemChanges {
            location: Some(moved.path().to_str().unwrap().into()),
            auto_capture: Some(false),
            ..Default::default()
        };
        let updated = pc.engine.update_system(&system.system_id, changes).unwrap();
        let zelda = pc.game("Zelda");
        assert_eq!(zelda.location, updated.location);
        assert!(!zelda.auto_capture);
        assert!(!pc.engine.has_uncaptured_changes(&zelda.bundle_id).unwrap());
        let bad = BundleChanges { location: Some(pc.loc().into()), ..Default::default() };
        assert!(pc.engine.update_bundle(&zelda.bundle_id, bad).is_err(), "move the system instead");

        pc.engine.remove_system(&system.system_id).unwrap();
        assert!(pc.names().is_empty());
        assert_eq!(pc.engine.remote_systems().unwrap().len(), 1, "can be set up again");
        assert_eq!(fs::read(moved.path().join("Zelda/save.bin")).unwrap(), b"z");
        pc.engine
            .add_system("Eden", moved.path().to_str().unwrap(), Some(system.system_id.clone()))
            .unwrap();
        assert_eq!(pc.engine.history(&zelda.bundle_id).unwrap().len(), 1);
        assert_eq!(pc.names(), vec!["Zelda"]);
    }
}
