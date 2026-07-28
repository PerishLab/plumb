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

pub fn write(path: &Path, ledger: &Ledger) -> Result<(), Error> {
    let text =
        serde_json::to_string_pretty(ledger).map_err(|error| Error::Parse(error.to_string()))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| Error::Write(parent.to_path_buf(), error.to_string()))?;
    }
    let beside = path.with_extension("part");
    fs::write(&beside, format!("{text}\n"))
        .map_err(|error| Error::Write(beside.clone(), error.to_string()))?;
    fs::rename(&beside, path).map_err(|error| Error::Write(path.to_path_buf(), error.to_string()))
}

pub fn keep(ledger: &mut Ledger, record: Record) {
    ledger.records.retain(|held| held.path != record.path);
    ledger.records.push(record);
}

pub fn held(ledger: &Ledger, path: &Path) -> bool {
    ledger.records.iter().any(|record| record.path == path)
}
