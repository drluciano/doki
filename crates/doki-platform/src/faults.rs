//! Fault injection for safety tests.
//!
//! [`FaultyLocation`] wraps another location and fails the Nth mutating call
//! (stage, commit, remove, discard). The engine treats any failure during an
//! install exactly like a crash: it stops and leaves the journal for
//! recovery. Tests therefore simulate a crash at every step by failing each
//! call index in turn.

use std::cell::Cell;
use std::io::{self, Read};
use std::rc::Rc;

use doki_core::error::StorageSide;
use doki_core::{BundlePath, Error, Result};

use crate::{Capabilities, SaveLocation};

/// Shared counter deciding which mutating call fails.
#[derive(Debug, Default)]
pub struct FaultPlan {
    calls: Cell<usize>,
    fail_at: Cell<Option<usize>>,
}

impl FaultPlan {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Fail the mutating call with this zero-based index, and every later one
    /// (a crashed process makes no further progress).
    pub fn fail_from(&self, index: usize) {
        self.fail_at.set(Some(index));
    }

    pub fn disarm(&self) {
        self.fail_at.set(None);
    }

    /// Mutating calls observed so far.
    pub fn calls(&self) -> usize {
        self.calls.get()
    }

    fn check(&self, what: &str) -> Result<()> {
        let n = self.calls.get();
        self.calls.set(n + 1);
        match self.fail_at.get() {
            Some(at) if n >= at => Err(Error::io(
                format!("injected fault at call {n} ({what})"),
                StorageSide::SaveLocation,
                io::Error::other("injected"),
            )),
            _ => Ok(()),
        }
    }
}

pub struct FaultyLocation<L> {
    inner: L,
    plan: Rc<FaultPlan>,
}

impl<L> FaultyLocation<L> {
    pub fn new(inner: L, plan: Rc<FaultPlan>) -> Self {
        FaultyLocation { inner, plan }
    }
}

impl<L: SaveLocation> SaveLocation for FaultyLocation<L> {
    fn describe(&self) -> String {
        self.inner.describe()
    }
    fn capabilities(&self) -> Capabilities {
        self.inner.capabilities()
    }
    fn check_available(&self) -> Result<()> {
        self.inner.check_available()
    }
    fn list(&self) -> Result<Vec<BundlePath>> {
        self.inner.list()
    }
    fn open(&self, path: &BundlePath) -> Result<Box<dyn Read + '_>> {
        self.inner.open(path)
    }
    fn stage(&self, op: &str, path: &BundlePath, data: &mut dyn Read) -> Result<()> {
        self.plan.check("stage")?;
        self.inner.stage(op, path, data)
    }
    fn open_staged(&self, op: &str, path: &BundlePath) -> Result<Box<dyn Read + '_>> {
        self.inner.open_staged(op, path)
    }
    fn commit_staged(&self, op: &str, path: &BundlePath) -> Result<()> {
        self.plan.check("commit")?;
        self.inner.commit_staged(op, path)
    }
    fn remove(&self, path: &BundlePath) -> Result<()> {
        self.plan.check("remove")?;
        self.inner.remove(path)
    }
    fn discard_staging(&self, op: &str) -> Result<()> {
        self.plan.check("discard")?;
        self.inner.discard_staging(op)
    }
}
