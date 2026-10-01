use std::{collections::BTreeMap, future::Future, sync::Mutex};

use serde::{Deserialize, Serialize};

use crate::{Error, Result, Rulebook, Snapshot, Values};

#[cfg(all(feature = "sql", not(target_arch = "wasm32")))]
mod sql;
#[cfg(all(feature = "sql", not(target_arch = "wasm32")))]
pub use sql::{LibsqlStore, SCHEMA};

/// Which immutable configuration revision to load.
#[derive(Clone, Copy, Debug)]
pub enum Selection {
    /// Latest scheduled revision, including future changes, for editing.
    Latest,
    /// Latest revision effective at these Unix seconds, for decisions.
    At(i64),
    /// Exact retained revision, for commitments already made under it.
    Revision(u64),
}

/// Caller-authorized prospective configuration change. Times concern policy only.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Change {
    /// Complete sparse configuration after this change.
    pub rulebook: Rulebook,
    /// Trusted service time when notice is announced.
    pub announced_at: i64,
    /// When new decisions begin using this revision.
    pub effective_at: i64,
    /// Minimum notice required by the authorized caller's policy.
    pub notice_seconds: u64,
    /// Caller-selected policy epoch; cannot decrease.
    pub policy_epoch: u64,
}

/// A committed immutable configuration; contains no member state or activity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Revision {
    /// Positive monotonically increasing revision.
    pub revision: u64,
    /// Announced configuration and its timing.
    pub change: Change,
}

impl Revision {
    /// Produce community-only flat content. Signing and key IDs belong to csgn.
    pub fn snapshot(&self, community: &str, issued: i64) -> Result<Snapshot> {
        community_key(community)?;
        if issued < self.change.effective_at {
            return Err(Error::Invalid("revision not effective"));
        }
        Ok(Snapshot {
            community: community.into(),
            kind: crate::SnapshotKind::Settings,
            revision: self.revision,
            policy_epoch: self.change.policy_epoch,
            issued,
            content: self.change.rulebook.resolve(&Values::new())?,
        })
    }
}

/// Small persistence boundary. Implementations must atomically validate and append,
/// compare the latest revision, preserve older revisions, and isolate communities.
pub trait Storage: Send + Sync {
    /// Read consistently, returning None when no revision matches.
    fn load(
        &self,
        community: &str,
        selection: Selection,
    ) -> impl Future<Output = Result<Option<Revision>>> + Send;
    /// Append after `expected` (None requires an empty namespace).
    /// Never retry a write automatically: transport errors may be ambiguous.
    fn append(
        &self,
        community: &str,
        expected: Option<u64>,
        change: Change,
    ) -> impl Future<Output = Result<Revision>> + Send;
}

/// Functional in-memory backend using the same validation as libSQL.
#[derive(Default)]
pub struct MemoryStore {
    communities: Mutex<BTreeMap<String, Vec<Revision>>>,
}

impl Storage for MemoryStore {
    async fn load(&self, community: &str, selection: Selection) -> Result<Option<Revision>> {
        community_key(community)?;
        validate_selection(selection)?;
        let communities = self.communities.lock().map_err(|_| Error::Storage)?;
        Ok(communities
            .get(community)
            .and_then(|revisions| {
                revisions.iter().rev().find(|r| match selection {
                    Selection::Latest => true,
                    Selection::At(time) => r.change.effective_at <= time,
                    Selection::Revision(revision) => r.revision == revision,
                })
            })
            .cloned())
    }

    async fn append(
        &self,
        community: &str,
        expected: Option<u64>,
        change: Change,
    ) -> Result<Revision> {
        community_key(community)?;
        let mut communities = self.communities.lock().map_err(|_| Error::Storage)?;
        let revisions = communities.entry(community.into()).or_default();
        let revision = prepare(revisions.last(), expected, change)?;
        revisions.push(revision.clone());
        Ok(revision)
    }
}

fn community_key(key: &str) -> Result<()> {
    if key.is_empty() || key.contains('\0') {
        return Err(Error::Invalid("community key"));
    }
    Ok(())
}

fn validate_selection(selection: Selection) -> Result<()> {
    match selection {
        Selection::At(t) if t < 0 => Err(Error::Invalid("negative time")),
        Selection::Revision(r) if r == 0 || r > i64::MAX as u64 => {
            Err(Error::Invalid("revision range"))
        }
        _ => Ok(()),
    }
}

fn prepare(previous: Option<&Revision>, expected: Option<u64>, change: Change) -> Result<Revision> {
    if previous.map(|r| r.revision) != expected {
        return Err(Error::Conflict);
    }
    change.rulebook.validate()?;
    let notice =
        i64::try_from(change.notice_seconds).map_err(|_| Error::Invalid("notice overflow"))?;
    let earliest = change
        .announced_at
        .checked_add(notice)
        .ok_or(Error::Invalid("notice overflow"))?;
    if change.announced_at < 0
        || change.effective_at < earliest
        || change.policy_epoch > i64::MAX as u64
    {
        return Err(Error::Invalid("notice or epoch"));
    }
    if let Some(previous) = previous {
        if change.effective_at <= previous.change.effective_at
            || change.announced_at < previous.change.announced_at
            || change.policy_epoch < previous.change.policy_epoch
        {
            return Err(Error::Invalid("nonprospective change"));
        }
        change.rulebook.follows(&previous.change.rulebook)?;
    }
    let revision = expected
        .unwrap_or(0)
        .checked_add(1)
        .filter(|r| *r <= i64::MAX as u64)
        .ok_or(Error::Invalid("revision overflow"))?;
    Ok(Revision { revision, change })
}
