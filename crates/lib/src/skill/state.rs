use super::Error;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const SCHEMA: u32 = 1;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Ledger {
    pub schema: u32,
    pub records: Vec<Record>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Record {
    pub agent: String,
    pub path: PathBuf,
    pub version: String,
    pub url: String,
    pub sha: String,
}

pub fn read(path: &Path) -> Result<Ledger, Error> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Ledger {
                schema: SCHEMA,
                records: Vec::new(),
            });
        }
        Err(error) => return Err(Error::Read(path.to_path_buf(), error.to_string())),
    };
    let ledger: Ledger =
        serde_json::from_str(&text).map_err(|error| Error::Parse(error.to_string()))?;
    if ledger.schema != SCHEMA {
        return Err(Error::Schema(ledger.schema));
    }
    Ok(ledger)
}

pub fn write(target: &Path, ledger: &Ledger) -> Result<(), Error> {
    let text =
        serde_json::to_string_pretty(ledger).map_err(|error| Error::Parse(error.to_string()))?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| Error::Write(parent.to_path_buf(), error.to_string()))?;
    }
    let beside = target.with_extension("part");
    private(&beside, format!("{text}\n").as_bytes())
        .map_err(|error| Error::Write(beside.clone(), error.to_string()))?;
    fs::rename(&beside, target)
        .map_err(|error| Error::Write(target.to_path_buf(), error.to_string()))
}

#[cfg(unix)]
fn private(seat: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(seat)?;
    file.set_permissions(fs::Permissions::from_mode(0o600))?;
    file.write_all(bytes)
}

#[cfg(not(unix))]
fn private(seat: &Path, bytes: &[u8]) -> std::io::Result<()> {
    fs::write(seat, bytes)
}

pub fn keep(ledger: &mut Ledger, record: Record) {
    ledger.records.retain(|held| held.path != record.path);
    ledger.records.push(record);
}

pub fn held(ledger: &Ledger, path: &Path) -> bool {
    ledger.records.iter().any(|record| record.path == path)
}
