//! Ordinary local folders (Windows, Linux, macOS desktop filesystems).

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use doki_core::error::StorageSide::SaveLocation as Side;
use doki_core::path::RESERVED_DIR;
use doki_core::{BundlePath, Error, Result};

use crate::{Capabilities, SaveLocation, Support};

/// A save folder on a local filesystem.
///
/// Per-file replacement uses write-to-staging, flush, then rename over the
/// target. Staging lives in `<root>/.doki-staging/<op>/` so the rename stays
/// on one volume. `std::fs::rename` replaces an existing file in one step on
/// Windows (MoveFileExW with MOVEFILE_REPLACE_EXISTING) and POSIX systems;
/// behaviour on network shares and removable FAT volumes is not established,
/// which is why a mixed multi-file state is always journaled by the engine.
#[derive(Debug, Clone)]
pub struct LocalDir {
    root: PathBuf,
}

impl LocalDir {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        LocalDir { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn target(&self, path: &BundlePath) -> PathBuf {
        path.segments().fold(self.root.clone(), |p, s| p.join(s))
    }

    fn staging_dir(&self, op: &str) -> PathBuf {
        self.root.join(RESERVED_DIR).join(op)
    }

    fn staged(&self, op: &str, path: &BundlePath) -> PathBuf {
        path.segments().fold(self.staging_dir(op), |p, s| p.join(s))
    }

    fn walk(&self, dir: &Path, prefix: &str, out: &mut Vec<BundlePath>) -> Result<()> {
        let entries = fs::read_dir(dir).map_err(|e| Error::io("listing save folder", Side, e))?;
        for entry in entries {
            let entry = entry.map_err(|e| Error::io("listing save folder", Side, e))?;
            let name = entry.file_name();
            let name = name.to_str().ok_or_else(|| {
                Error::InvalidInput(format!("file name is not valid Unicode: {name:?}"))
            })?;
            if prefix.is_empty() && name == RESERVED_DIR {
                continue;
            }
            let rel = if prefix.is_empty() { name.to_owned() } else { format!("{prefix}/{name}") };
            let kind = entry.file_type().map_err(|e| Error::io("reading file type", Side, e))?;
            if kind.is_symlink() {
                // Following links could read or overwrite files outside the
                // folder the user granted.
                return Err(Error::InvalidInput(format!(
                    "links are not supported in saves: {rel}"
                )));
            } else if kind.is_dir() {
                self.walk(&entry.path(), &rel, out)?;
            } else if kind.is_file() {
                out.push(BundlePath::new(&rel)?);
            }
        }
        Ok(())
    }
}

fn sync_dir(dir: &Path) {
    // Directory fsync persists the rename on POSIX. Windows has no
    // equivalent through std; NTFS journals the metadata itself.
    #[cfg(unix)]
    if let Ok(d) = File::open(dir) {
        let _ = d.sync_all();
    }
    #[cfg(not(unix))]
    let _ = dir;
}

impl SaveLocation for LocalDir {
    fn describe(&self) -> String {
        self.root.display().to_string()
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            per_file_atomic_replace: Support::Supported,
            durable_flush: Support::Supported,
        }
    }

    fn check_available(&self) -> Result<()> {
        match fs::metadata(&self.root) {
            Ok(m) if m.is_dir() => Ok(()),
            Ok(_) => Err(Error::SourceUnavailable(format!("{} is not a folder", self.describe()))),
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
                Err(Error::PermissionLost(self.describe()))
            }
            Err(_) => Err(Error::SourceUnavailable(self.describe())),
        }
    }

    fn list(&self) -> Result<Vec<BundlePath>> {
        self.check_available()?;
        let mut out = Vec::new();
        self.walk(&self.root, "", &mut out)?;
        out.sort();
        Ok(out)
    }

    fn open(&self, path: &BundlePath) -> Result<Box<dyn Read + '_>> {
        let f = File::open(self.target(path))
            .map_err(|e| Error::io(format!("opening {path}"), Side, e))?;
        Ok(Box::new(f))
    }

    fn stage(&self, op: &str, path: &BundlePath, data: &mut dyn Read) -> Result<()> {
        let dest = self.staged(op, path);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| Error::io("creating staging folder", Side, e))?;
        }
        let mut f = File::create(&dest).map_err(|e| Error::io("creating staged file", Side, e))?;
        io::copy(data, &mut f).map_err(|e| Error::io(format!("staging {path}"), Side, e))?;
        f.flush().and_then(|_| f.sync_all()).map_err(|e| Error::io("flushing staged file", Side, e))
    }

    fn open_staged(&self, op: &str, path: &BundlePath) -> Result<Box<dyn Read + '_>> {
        let f = File::open(self.staged(op, path))
            .map_err(|e| Error::io(format!("opening staged {path}"), Side, e))?;
        Ok(Box::new(f))
    }

    fn commit_staged(&self, op: &str, path: &BundlePath) -> Result<()> {
        let dest = self.target(path);
        let parent = dest.parent().expect("bundle paths are relative to the root").to_owned();
        fs::create_dir_all(&parent).map_err(|e| Error::io("creating save subfolder", Side, e))?;
        fs::rename(self.staged(op, path), &dest)
            .map_err(|e| Error::io(format!("replacing {path}"), Side, e))?;
        sync_dir(&parent);
        Ok(())
    }

    fn remove(&self, path: &BundlePath) -> Result<()> {
        let dest = self.target(path);
        match fs::remove_file(&dest) {
            Ok(()) => {
                if let Some(parent) = dest.parent() {
                    sync_dir(parent);
                }
                Ok(())
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(Error::io(format!("removing {path}"), Side, e)),
        }
    }

    fn discard_staging(&self, op: &str) -> Result<()> {
        let dir = self.staging_dir(op);
        match fs::remove_dir_all(&dir) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(Error::io("removing staging folder", Side, e)),
        }
        // Remove the shared staging root once empty so the save folder looks
        // untouched; ignore failure (another op may be using it).
        let _ = fs::remove_dir(self.root.join(RESERVED_DIR));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> BundlePath {
        BundlePath::new(s).unwrap()
    }

    #[test]
    fn missing_folder_is_unavailable_not_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let loc = LocalDir::new(tmp.path().join("gone"));
        assert_eq!(loc.list().unwrap_err().code(), "SourceUnavailable");
    }

    #[test]
    fn stage_commit_remove_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let loc = LocalDir::new(tmp.path());
        fs::write(tmp.path().join("a.sav"), b"old").unwrap();

        loc.stage("op1", &p("a.sav"), &mut &b"new"[..]).unwrap();
        loc.stage("op1", &p("sub/b.dat"), &mut &b"bee"[..]).unwrap();
        assert_eq!(loc.list().unwrap(), vec![p("a.sav")], "staging is not part of the bundle");
        assert_eq!(fs::read(tmp.path().join("a.sav")).unwrap(), b"old", "staging is not install");

        loc.commit_staged("op1", &p("a.sav")).unwrap();
        loc.commit_staged("op1", &p("sub/b.dat")).unwrap();
        assert_eq!(fs::read(tmp.path().join("a.sav")).unwrap(), b"new");
        assert_eq!(loc.list().unwrap(), vec![p("a.sav"), p("sub/b.dat")]);

        loc.remove(&p("sub/b.dat")).unwrap();
        loc.remove(&p("sub/b.dat")).unwrap();
        loc.discard_staging("op1").unwrap();
        assert_eq!(loc.list().unwrap(), vec![p("a.sav")]);
        assert!(!tmp.path().join(RESERVED_DIR).exists());
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_are_refused() {
        let tmp = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink("/etc/hostname", tmp.path().join("link")).unwrap();
        assert_eq!(LocalDir::new(tmp.path()).list().unwrap_err().code(), "InvalidInput");
    }
}
