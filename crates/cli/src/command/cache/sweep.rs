use super::bucket::{self, Record};
use super::disk::Disk;
use super::graph;
use super::plan::{self, Plan, Seat};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs::TryLockError;
use std::path::{Path, PathBuf};

pub(super) const IDLE: u64 = 7 * 24 * 60 * 60;

#[derive(Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum Status {
    Live,
    Idle,
    Abandoned,
    Active,
    Unknown,
}

#[derive(Serialize)]
pub(super) struct Entry {
    pub bucket: String,
    pub identity: Option<String>,
    pub status: Status,
    pub used: Option<u64>,
    pub bytes: u64,
    pub working: u64,
    pub superseded: u64,
    pub targets: Vec<String>,
    pub reclaimable: u64,
    pub reclaimed: u64,
    pub refusal: Option<String>,
}

impl Entry {
    fn refuse(mut self, reason: &str) -> Self {
        self.refusal = Some(reason.to_string());
        self
    }
}

#[derive(Clone, Copy)]
pub(super) struct Sweep {
    pub apply: bool,
    pub span: u64,
    pub operator: bool,
}

pub(super) fn survey(home: &Path, bucket: &Path, now: u64, sweep: Sweep) -> Entry {
    let entry = Entry {
        bucket: graph::Node(bucket).name(),
        identity: bucket::identity(bucket),
        status: Status::Unknown,
        used: None,
        bytes: graph::Node(bucket).size(),
        working: 0,
        superseded: 0,
        targets: Vec::new(),
        reclaimable: 0,
        reclaimed: 0,
        refusal: None,
    };
    let Some(identity) = entry.identity.clone() else {
        if !sweep.operator {
            return entry.refuse("no Plumb marker; only plumb cache reclaim --apply removes it");
        }
        return legacy(bucket, sweep, entry);
    };
    let path = bucket::ledger(home, bucket);
    let record = match bucket::read(&path) {
        Ok(Some(record)) if record.identity == identity => record,
        Ok(Some(_)) => return entry.refuse("registry record names another identity"),
        Ok(None) => return entry.refuse("no registry record; Plumb never reclaims it"),
        Err(error) => return entry.refuse(&error),
    };
    let lock = match bucket::lease(bucket) {
        Ok(lock) => lock,
        Err(error) => return entry.refuse(&error),
    };
    match lock.try_lock() {
        Ok(()) => held(
            Held {
                path,
                record,
                sweep,
            },
            bucket,
            now,
            entry,
        ),
        Err(TryLockError::WouldBlock) => Entry {
            status: Status::Active,
            used: Some(record.used),
            ..entry
        },
        Err(TryLockError::Error(error)) => entry.refuse(&format!("cannot test lease: {error}")),
    }
}

struct Held {
    path: PathBuf,
    record: Record,
    sweep: Sweep,
}

fn legacy(bucket: &Path, sweep: Sweep, mut entry: Entry) -> Entry {
    let lock = match bucket::lease(bucket) {
        Ok(lock) => lock,
        Err(error) => return entry.refuse(&error),
    };
    match lock.try_lock() {
        Ok(()) => {}
        Err(TryLockError::WouldBlock) => {
            entry.status = Status::Active;
            return entry;
        }
        Err(TryLockError::Error(error)) => {
            return entry.refuse(&format!("cannot test lease: {error}"));
        }
    }
    let plan = match Seat(bucket).whole() {
        Ok(plan) => plan,
        Err(error) => return entry.refuse(&error),
    };
    tally(bucket, &plan, &mut entry);
    if sweep.apply {
        let (freed, failure) = plan::execute(&plan.targets, &mut Record::new(""));
        entry.reclaimed = freed;
        if let Some(error) = failure {
            return entry.refuse(&error);
        }
    }
    entry
}

fn tally(bucket: &Path, plan: &Plan, entry: &mut Entry) {
    entry.working = plan.working;
    entry.superseded = plan.superseded;
    for target in &plan.targets {
        entry.reclaimable += target.bytes;
        for path in &target.paths {
            entry.targets.push(Seat(bucket).relative(path));
        }
    }
}

fn held(mut held: Held, bucket: &Path, now: u64, mut entry: Entry) -> Entry {
    entry.used = Some(held.record.used);
    entry.status = if !Path::new(&held.record.identity).exists() {
        Status::Abandoned
    } else if now.saturating_sub(held.record.used) > IDLE {
        Status::Idle
    } else {
        Status::Live
    };
    let plan = if entry.status == Status::Live {
        Seat(bucket).live(&held.record, now, held.sweep.span)
    } else {
        Seat(bucket).whole()
    };
    let plan = match plan {
        Ok(plan) => plan,
        Err(error) => return entry.refuse(&error),
    };
    tally(bucket, &plan, &mut entry);
    if !held.sweep.apply {
        return entry;
    }
    let (freed, failure) = plan::execute(&plan.targets, &mut held.record);
    if entry.status != Status::Live && failure.is_none() {
        held.record.units.clear();
    }
    entry.reclaimed = freed;
    if let Err(error) = bucket::write(&held.path, &held.record) {
        return entry.refuse(&error);
    }
    match failure {
        Some(error) => entry.refuse(&error),
        None => entry,
    }
}

pub(crate) fn settle(bucket: &Path, identity: &str, since: u64) -> Result<u64, String> {
    let home = bucket::home()?;
    let path = bucket::ledger(&home, bucket);
    let now = now();
    let mut record = bucket::read(&path)?.unwrap_or_else(|| Record::new(identity));
    if record.identity != identity {
        return Err(format!("{} names another identity", path.display()));
    }
    let mut seen = BTreeMap::new();
    for profile in graph::profiles(bucket)? {
        let live = profile.closure(since);
        let mut stamp = |key: String, fresh: bool| {
            let used = record.units.get(&key).copied().filter(|_| !fresh);
            seen.insert(key, used.unwrap_or(now));
        };
        for key in profile.units.keys() {
            stamp(format!("{}/{key}", profile.name), live.contains(key));
        }
        for (key, _, fresh) in profile.incremental()? {
            stamp(format!("{}/{key}", profile.name), fresh >= since);
        }
    }
    record.units = seen;
    record.used = now;
    let span = Disk::read(bucket)?.span();
    let plan = Seat(bucket).live(&record, now, span)?;
    let (freed, failure) = plan::execute(&plan.targets, &mut record);
    bucket::write(&path, &record)?;
    failure.map_or(Ok(freed), Err)
}

pub(crate) fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
