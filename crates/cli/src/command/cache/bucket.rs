use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

pub(super) const SCHEMA: &str = "plumb.guard-cache/v1";
const MARKER: &str = "plumb.json";

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Marker {
    schema: String,
    keeper: String,
    identity: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Record {
    schema: String,
    pub identity: String,
    pub used: u64,
    pub units: BTreeMap<String, u64>,
}

impl Record {
    pub fn new(identity: &str) -> Self {
        Self {
            schema: SCHEMA.to_string(),
            identity: identity.to_string(),
            used: 0,
            units: BTreeMap::new(),
        }
    }
}

pub(crate) fn home() -> Result<PathBuf, String> {
    plumb::config::value("PLUMB_HOME")
        .map(PathBuf::from)
        .or_else(|| plumb::config::data("plumb"))
        .ok_or_else(|| "cannot cache Guard compilation: no PLUMB_HOME".to_string())
}

pub(super) fn root(home: &Path) -> PathBuf {
    if cfg!(windows) {
        home.join("c")
    } else {
        home.join("cache").join("guard").join("cargo")
    }
}

pub(crate) fn seat(home: &Path, identity: &str) -> PathBuf {
    let digest = plumb::depot::sha(identity.as_bytes());
    if cfg!(windows) {
        root(home).join(&digest[..32])
    } else {
        root(home).join(digest)
    }
}

pub(super) fn ledger(home: &Path, bucket: &Path) -> PathBuf {
    home.join("state")
        .join("guard")
        .join("cargo")
        .join(format!("{}.json", super::graph::Node(bucket).name()))
}

pub(super) fn identity(bucket: &Path) -> Option<String> {
    let bytes = std::fs::read(bucket.join(MARKER)).ok()?;
    let marker: Marker = serde_json::from_slice(&bytes).ok()?;
    (marker.schema == SCHEMA && marker.keeper == "plumb").then_some(marker.identity)
}

pub(crate) fn adopt(bucket: &Path, identity: &str) -> Result<(), String> {
    match self::identity(bucket) {
        Some(held) if held == identity => Ok(()),
        Some(held) => Err(format!(
            "Guard cache {} belongs to {held}, not {identity}",
            bucket.display()
        )),
        None => {
            let marker = Marker {
                schema: SCHEMA.to_string(),
                keeper: "plumb".to_string(),
                identity: identity.to_string(),
            };
            replace(
                &bucket.join(MARKER),
                &serde_json::to_vec_pretty(&marker).map_err(|error| error.to_string())?,
            )
        }
    }
}

pub(super) fn read(path: &Path) -> Result<Option<Record>, String> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("cannot read {}: {error}", path.display())),
    };
    let record: Record = serde_json::from_slice(&bytes)
        .map_err(|error| format!("cannot parse {}: {error}", path.display()))?;
    if record.schema != SCHEMA {
        return Err(format!(
            "{} has unknown schema {}",
            path.display(),
            record.schema
        ));
    }
    Ok(Some(record))
}

pub(super) fn write(path: &Path, record: &Record) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    replace(
        path,
        &serde_json::to_vec_pretty(record).map_err(|error| error.to_string())?,
    )
}

fn replace(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let beside = path.with_extension("part");
    std::fs::write(&beside, bytes)
        .map_err(|error| format!("cannot write {}: {error}", beside.display()))?;
    std::fs::rename(&beside, path)
        .map_err(|error| format!("cannot replace {}: {error}", path.display()))
}

pub(crate) fn lease(bucket: &Path) -> Result<File, String> {
    let path = bucket.join("lease");
    if let Ok(metadata) = std::fs::symlink_metadata(&path)
        && !metadata.is_file()
    {
        return Err(format!(
            "Guard compilation lease is not a regular file: {}",
            path.display()
        ));
    }
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .map_err(|error| format!("cannot open Guard compilation lease: {error}"))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn bounded() {
        let home = Path::new("C:/Users/operator/AppData/Local/plumb");
        let first = super::seat(home, &"repository/".repeat(100));
        let second = super::seat(home, &"repository/".repeat(99));
        assert_ne!(first, second);
        if cfg!(windows) {
            assert_eq!(first.parent(), Some(home.join("c").as_path()));
            assert_eq!(first.file_name().unwrap().len(), 32);
            assert_eq!(first.as_os_str().len(), home.as_os_str().len() + 35);
        }
    }
}
