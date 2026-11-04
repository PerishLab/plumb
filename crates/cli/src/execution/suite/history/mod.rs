#[cfg(test)]
mod tests;

use super::model::{Program, Schedule};
use super::queue::Completed;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const SCHEMA: &str = "plumb.test-timing/v1";

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    key: String,
    milliseconds: u64,
    threads: usize,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema: String,
    keeper: String,
    root: PathBuf,
    cores: usize,
    entries: Vec<Entry>,
}

pub(super) struct History {
    path: PathBuf,
    root: PathBuf,
    cores: usize,
    keys: Vec<String>,
    original: Option<Vec<u8>>,
}

impl History {
    pub fn prepare(
        root: &Path,
        home: &Path,
        cores: usize,
        programs: &[Program],
    ) -> (Option<Self>, Schedule) {
        let prepared = Self::new(root, home, cores, programs)
            .and_then(|history| history.schedule().map(|schedule| (history, schedule)));
        match prepared {
            Ok((history, schedule)) => (Some(history), schedule),
            Err(error) => {
                refused("cold execution", &error);
                (None, Schedule::cold(cores, programs.len()))
            }
        }
    }

    pub fn new(
        root: &Path,
        home: &Path,
        cores: usize,
        programs: &[Program],
    ) -> Result<Self, String> {
        if !home.is_absolute() {
            return Err("test timing history requires an absolute Plumb home".into());
        }
        let root = root.canonicalize().map_err(|error| error.to_string())?;
        let home = match home.try_exists().map_err(|error| error.to_string())? {
            true => home.canonicalize().map_err(|error| error.to_string())?,
            false => home.to_path_buf(),
        };
        let identity = serde_json::to_vec(&root).map_err(|error| error.to_string())?;
        let path = home
            .join("state/guard/test")
            .join(format!("{}.json", plumb::depot::sha(&identity)));
        let keys = programs
            .iter()
            .map(|program| program.key(&root))
            .collect::<Result<Vec<_>, _>>()?;
        if keys.iter().collect::<BTreeSet<_>>().len() != keys.len() {
            return Err("test timing targets have duplicate logical identities".into());
        }
        let original = read(&path)?;
        Ok(Self {
            path,
            root,
            cores: cores.max(1),
            keys,
            original,
        })
    }

    pub fn schedule(&self) -> Result<Schedule, String> {
        let cold = || Schedule::cold(self.cores, self.keys.len());
        let Some(bytes) = &self.original else {
            return Ok(cold());
        };
        let record: Record = serde_json::from_slice(bytes).map_err(|error| {
            format!(
                "cannot read test timing history; preserve {}: {error}",
                self.path.display()
            )
        })?;
        if record.schema != SCHEMA || record.keeper != "plumb" || record.root != self.root {
            return Err(format!(
                "test timing history has unknown ownership or schema; preserve {}",
                self.path.display()
            ));
        }
        let mut work = BTreeMap::new();
        for entry in record.entries {
            if entry.threads == 0
                || entry.threads > record.cores
                || work
                    .insert(
                        entry.key,
                        entry
                            .milliseconds
                            .saturating_mul(entry.threads as u64)
                            .max(1),
                    )
                    .is_some()
            {
                return Err(format!(
                    "test timing history has invalid or duplicate measurements; preserve {}",
                    self.path.display()
                ));
            }
        }
        if record.cores != self.cores || work.len() != self.keys.len() {
            return Ok(cold());
        }
        let weights = self
            .keys
            .iter()
            .map(|key| work.get(key).copied())
            .collect::<Option<Vec<_>>>();
        Ok(weights.map_or_else(cold, |weights| Schedule::weighted(self.cores, &weights)))
    }

    pub fn record(&self, completed: &[Option<Completed>]) -> Result<(), String> {
        self.schedule()?;
        if completed.len() != self.keys.len() {
            return Err("test timing history lacks the complete program set".into());
        }
        let entries = completed
            .iter()
            .enumerate()
            .map(|(index, result)| {
                let result = result
                    .as_ref()
                    .filter(|result| result.result.is_ok() && result.index == index)
                    .ok_or("test timing history requires successful actual execution")?;
                Ok(Entry {
                    key: self.keys[index].clone(),
                    milliseconds: result.elapsed.as_millis().try_into().unwrap_or(u64::MAX),
                    threads: result.threads,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let record = Record {
            schema: SCHEMA.into(),
            keeper: "plumb".into(),
            root: self.root.clone(),
            cores: self.cores,
            entries,
        };
        if read(&self.path)? != self.original {
            return Err(
                "test timing history changed during execution; preserve both observations".into(),
            );
        }
        let bytes = serde_json::to_vec(&record).map_err(|error| error.to_string())?;
        if bytes.len() > 1024 * 1024 {
            return Err("test timing history exceeds 1 MiB".into());
        }
        plumb::replace::write(&self.path, &bytes).map_err(|error| error.to_string())
    }
}

pub(super) fn refused(outcome: &str, reason: &str) {
    eprintln!("guard/test: timing advice refused ({outcome}): {reason}");
    crate::diagnostic::event(
        "test.timing",
        || serde_json::json!({"outcome":outcome,"reason":reason}),
    );
}

fn read(path: &Path) -> Result<Option<Vec<u8>>, String> {
    for ancestor in path.ancestors().skip(1) {
        match std::fs::symlink_metadata(ancestor) {
            Ok(metadata) if !metadata.is_dir() || metadata.file_type().is_symlink() => {
                return Err(format!(
                    "test timing history has a foreign parent: {}",
                    ancestor.display()
                ));
            }
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                return Err(error.to_string());
            }
            _ => (),
        }
    }
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.to_string()),
        Ok(metadata)
            if !metadata.is_file()
                || metadata.file_type().is_symlink()
                || metadata.len() > 1024 * 1024 =>
        {
            Err(format!(
                "test timing history is foreign or oversized; preserve {}",
                path.display()
            ))
        }
        Ok(_) => std::fs::read(path)
            .map(Some)
            .map_err(|error| error.to_string()),
    }
}
