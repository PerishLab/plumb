use super::Error;
use crate::{record, replace};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const SCHEMA: u32 = 1;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Ledger {
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
    record::read(path, SCHEMA)
        .map(Option::unwrap_or_default)
        .map_err(fault)
}

pub fn write(target: &Path, ledger: &Ledger) -> Result<(), Error> {
    let bytes = record::encode(SCHEMA, ledger).map_err(|error| Error::Parse(error.to_string()))?;
    replace::private(target, &bytes)
        .map_err(|error| Error::Write(target.to_path_buf(), error.to_string()))
}

pub(super) fn fault(error: record::Error) -> Error {
    match error {
        record::Error::Read(path, why) => Error::Read(path, why.to_string()),
        record::Error::Parse(_, why) => Error::Parse(why.to_string()),
        record::Error::Absent(path) => Error::Parse(format!("{} names no schema", path.display())),
        record::Error::Schema(_, seen) => Error::Schema(seen),
        record::Error::Write(path, why) => Error::Write(path, why.to_string()),
    }
}

pub fn keep(ledger: &mut Ledger, record: Record) {
    ledger.records.retain(|held| held.path != record.path);
    ledger.records.push(record);
}

pub fn held(ledger: &Ledger, path: &Path) -> bool {
    ledger.records.iter().any(|record| record.path == path)
}
