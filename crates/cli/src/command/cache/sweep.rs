use super::bucket::{self, Record};
use super::graph;
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

struct Target {
    key: Option<String>,
    paths: Vec<PathBuf>,
}

pub(super) fn survey(home: &Path, bucket: &Path, now: u64, apply: bool) -> Entry {
    let entry = Entry {
        bucket: graph::Node(bucket).name(),
        identity: bucket::identity(bucket),
        status: Status::Unknown,
        used: None,
        bytes: graph::Node(bucket).size(),
        targets: Vec::new(),
        reclaimable: 0,
        reclaimed: 0,
        refusal: None,
    };
    let Some(identity) = entry.identity.clone() else {
        return entry.refuse("no Plumb marker; Plumb never reclaims an unmarked cache");
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
                apply,
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
    apply: bool,
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
    let targets = match plan(bucket, &held.record, entry.status, now) {
        Ok(targets) => targets,
        Err(error) => return entry.refuse(&error),
    };
    for target in &targets {
        for path in &target.paths {
            entry.reclaimable += graph::Node(path).size();
            entry.targets.push(relative(bucket, path));
        }
    }
    if !held.apply {
        return entry;
    }
    let (freed, failure) = execute(&targets, &mut held.record);
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

fn plan(bucket: &Path, record: &Record, status: Status, now: u64) -> Result<Vec<Target>, String> {
    if status != Status::Live {
        return Ok(graph::Node(bucket)
            .listing()?
            .into_iter()
            .filter(|path| !matches!(graph::Node(path).name().as_str(), "lease" | "plumb.json"))
            .map(|path| Target {
                key: None,
                paths: vec![path],
            })
            .collect());
    }
    let idle = |key: &str| {
        record
            .units
            .get(key)
            .is_some_and(|used| now.saturating_sub(*used) > IDLE)
    };
    let mut targets = Vec::new();
    for profile in graph::profiles(bucket)? {
        for key in profile.units.keys() {
            let full = format!("{}/{key}", profile.name);
            if idle(&full) {
                targets.push(Target {
                    paths: profile.files(key)?,
                    key: Some(full),
                });
            }
        }
        for (key, path, _) in profile.incremental()? {
            let full = format!("{}/{key}", profile.name);
            if idle(&full) {
                targets.push(Target {
                    key: Some(full),
                    paths: vec![path],
                });
            }
        }
        targets.extend(profile.orphans()?.into_iter().map(|path| Target {
            key: None,
            paths: vec![path],
        }));
    }
    Ok(targets)
}

fn execute(targets: &[Target], record: &mut Record) -> (u64, Option<String>) {
    let mut freed = 0;
    for target in targets {
        for path in &target.paths {
            let bytes = graph::Node(path).size();
            if let Err(error) = remove(path) {
                return (
                    freed,
                    Some(format!("cannot remove {}: {error}", path.display())),
                );
            }
            freed += bytes;
        }
        if let Some(key) = &target.key {
            record.units.remove(key);
        }
    }
    (freed, None)
}

fn remove(path: &Path) -> std::io::Result<()> {
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    }
}

fn relative(bucket: &Path, path: &Path) -> String {
    path.strip_prefix(bucket)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
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
    let targets = plan(bucket, &record, Status::Live, now)?;
    let (freed, failure) = execute(&targets, &mut record);
    bucket::write(&path, &record)?;
    failure.map_or(Ok(freed), Err)
}

pub(crate) fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
