//! One game inside a whole-system save folder.
//!
//! Many emulators keep every game's save in one folder: either a subfolder
//! per game (Eden, Dolphin, PPSSPP) or loose files named after the game
//! (`Pokemon Emerald.sav`, `Pokemon Emerald.ss1`; RetroArch, mGBA). A
//! [`MemberLocation`] presents one such game as its own save location, so
//! the engine tracks, checkpoints and installs it exactly like a game whose
//! folder was chosen on its own. Paths inside a checkpoint are relative to
//! the member, so the same game in the same emulator has the same paths on
//! every device.
//!
//! [`discover`] finds the members of a system folder.

use std::io::Read;

use doki_core::{BundlePath, Result};

use crate::{Capabilities, SaveLocation};

/// Which part of a system folder belongs to one game.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Member {
    /// Everything inside the subfolder with this name.
    Folder(String),
    /// Loose files directly in the system folder named `<stem>` or
    /// `<stem>.<extensions>`, where the extensions have no spaces (so
    /// `Super Mario Bros.srm` does not own `Super Mario Bros. 3.srm`).
    Files(String),
}

impl Member {
    /// Name shown for a newly discovered game.
    pub fn name(&self) -> &str {
        match self {
            Member::Folder(n) | Member::Files(n) => n,
        }
    }

    fn owns_file(stem: &str, name: &str) -> bool {
        name == stem
            || name
                .strip_prefix(stem)
                .is_some_and(|rest| rest.starts_with('.') && !rest.contains(char::is_whitespace))
    }
}

/// Names that are never a game: hidden files and the litter operating
/// systems leave in folders.
fn ignored(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    name.starts_with('.') || matches!(lower.as_str(), "desktop.ini" | "thumbs.db")
}

/// The games in a system folder, from its [`SaveLocation::entries`].
///
/// Each subfolder is one game. Loose files are grouped by name without the
/// last extension, and a group whose name extends another by a dot joins
/// it, so `Game.srm`, `Game.state` and `Game.state.auto` are one game.
pub fn discover(entries: &[String]) -> Vec<Member> {
    let mut out = Vec::new();
    let mut stems: Vec<String> = Vec::new();
    for entry in entries {
        if let Some(dir) = entry.strip_suffix('/') {
            if !ignored(dir) {
                out.push(Member::Folder(dir.to_owned()));
            }
        } else if !ignored(entry) {
            let stem = entry.rsplit_once('.').map_or(entry.as_str(), |(s, _)| s);
            if !stem.is_empty() {
                stems.push(stem.to_owned());
            }
        }
    }
    stems.sort();
    stems.dedup();
    // Sorted, so a shorter owning stem always comes first.
    let mut kept: Vec<String> = Vec::new();
    for stem in stems {
        if !kept.iter().any(|k| Member::owns_file(k, &stem)) {
            kept.push(stem);
        }
    }
    out.extend(kept.into_iter().map(Member::Files));
    out.sort();
    out
}

/// One member of a system folder, seen as a save location of its own.
///
/// Mutating calls go to the system folder with the member's prefix, so
/// staging and replacement keep the system adapter's guarantees. A member
/// folder that does not exist yet (a game only played on another device)
/// is empty, not missing; installing into it creates it. A missing system
/// folder is still `SourceUnavailable`.
pub struct MemberLocation<'a> {
    system: Box<dyn SaveLocation + 'a>,
    member: Member,
}

impl<'a> MemberLocation<'a> {
    pub fn new(system: Box<dyn SaveLocation + 'a>, member: Member) -> Self {
        MemberLocation { system, member }
    }

    /// The path in the system folder of a member path.
    fn outer(&self, path: &BundlePath) -> Result<BundlePath> {
        match &self.member {
            Member::Folder(dir) => BundlePath::new(&format!("{dir}/{}", path.as_str())),
            Member::Files(stem) => {
                if path.as_str().contains('/') || !Member::owns_file(stem, path.as_str()) {
                    // A received checkpoint can only hold this game's files.
                    return Err(doki_core::Error::InvalidInput(format!(
                        "{path} does not belong to {stem}"
                    )));
                }
                Ok(path.clone())
            }
        }
    }
}

impl SaveLocation for MemberLocation<'_> {
    fn describe(&self) -> String {
        format!("{} in {}", self.member.name(), self.system.describe())
    }

    fn capabilities(&self) -> Capabilities {
        self.system.capabilities()
    }

    fn check_available(&self) -> Result<()> {
        self.system.check_available()
    }

    fn list(&self) -> Result<Vec<BundlePath>> {
        match &self.member {
            Member::Folder(dir) => {
                let prefix = format!("{dir}/");
                self.system
                    .list_under(&BundlePath::new(dir)?)?
                    .iter()
                    .map(|p| BundlePath::new(&p.as_str()[prefix.len()..]))
                    .collect()
            }
            Member::Files(stem) => {
                let mut out = Vec::new();
                for entry in self.system.entries()? {
                    if !entry.ends_with('/') && Member::owns_file(stem, &entry) {
                        out.push(BundlePath::new(&entry)?);
                    }
                }
                Ok(out)
            }
        }
    }

    fn open(&self, path: &BundlePath) -> Result<Box<dyn Read + '_>> {
        self.system.open(&self.outer(path)?)
    }

    fn stage(&self, op: &str, path: &BundlePath, data: &mut dyn Read) -> Result<()> {
        self.system.stage(op, &self.outer(path)?, data)
    }

    fn open_staged(&self, op: &str, path: &BundlePath) -> Result<Box<dyn Read + '_>> {
        self.system.open_staged(op, &self.outer(path)?)
    }

    fn commit_staged(&self, op: &str, path: &BundlePath) -> Result<()> {
        self.system.commit_staged(op, &self.outer(path)?)
    }

    fn remove(&self, path: &BundlePath) -> Result<()> {
        self.system.remove(&self.outer(path)?)
    }

    fn discard_staging(&self, op: &str) -> Result<()> {
        // Operation ids are unique, so this only touches this game's staging.
        self.system.discard_staging(op)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::local::LocalDir;

    fn p(s: &str) -> BundlePath {
        BundlePath::new(s).unwrap()
    }

    fn strings(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn discovers_folders_and_groups_loose_files() {
        let found = discover(&strings(&[
            ".hidden",
            "0100F2C0115B6000/",
            "Game.srm",
            "Game.state",
            "Game.state.auto",
            "Super Mario Bros. 3.srm",
            "Super Mario Bros.srm",
            "Thumbs.db",
            "Zelda/",
        ]));
        assert_eq!(
            found,
            vec![
                Member::Folder("0100F2C0115B6000".into()),
                Member::Folder("Zelda".into()),
                Member::Files("Game".into()),
                Member::Files("Super Mario Bros".into()),
                Member::Files("Super Mario Bros. 3".into()),
            ]
        );
    }

    #[test]
    fn folder_member_maps_paths_and_creates_its_folder() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("A/sub")).unwrap();
        fs::write(tmp.path().join("A/sub/x.sav"), b"a").unwrap();
        fs::write(tmp.path().join("loose.sav"), b"l").unwrap();

        let a =
            MemberLocation::new(Box::new(LocalDir::new(tmp.path())), Member::Folder("A".into()));
        assert_eq!(a.list().unwrap(), vec![p("sub/x.sav")]);

        let b =
            MemberLocation::new(Box::new(LocalDir::new(tmp.path())), Member::Folder("B".into()));
        assert_eq!(b.list().unwrap(), Vec::<BundlePath>::new(), "not played here yet");
        b.stage("op", &p("y.sav"), &mut &b"new"[..]).unwrap();
        b.commit_staged("op", &p("y.sav")).unwrap();
        b.discard_staging("op").unwrap();
        assert_eq!(fs::read(tmp.path().join("B/y.sav")).unwrap(), b"new");
        assert_eq!(b.list().unwrap(), vec![p("y.sav")]);
    }

    #[test]
    fn files_member_sees_only_its_own_files() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("Game.srm"), b"1").unwrap();
        fs::write(tmp.path().join("Game.state.auto"), b"2").unwrap();
        fs::write(tmp.path().join("Gamer.srm"), b"3").unwrap();
        let m =
            MemberLocation::new(Box::new(LocalDir::new(tmp.path())), Member::Files("Game".into()));
        assert_eq!(m.list().unwrap(), vec![p("Game.srm"), p("Game.state.auto")]);
        assert!(m.open(&p("Gamer.srm")).is_err(), "another game's file");
        assert!(m.stage("op", &p("sub/Game.srm"), &mut &b""[..]).is_err());
    }

    #[test]
    fn missing_system_folder_is_unavailable() {
        let tmp = tempfile::tempdir().unwrap();
        let m = MemberLocation::new(
            Box::new(LocalDir::new(tmp.path().join("gone"))),
            Member::Folder("A".into()),
        );
        assert_eq!(m.list().unwrap_err().code(), "SourceUnavailable");
    }
}
