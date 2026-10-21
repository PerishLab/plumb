mod pack;
#[cfg(test)]
mod tests;
mod unpack;

use super::transport::stamp::Stamp;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const SCHEMA: &str = "plumb.guard-parcel/v1";

#[derive(Default, Serialize)]
pub(super) struct Receipt {
    pub cargo: bool,
    pub compiler: bool,
}

impl Receipt {
    pub fn any(&self) -> bool {
        self.cargo || self.compiler
    }
}

pub(super) struct Plan {
    pub identity: String,
    pub cargo: Option<PathBuf>,
    pub compiler: Option<PathBuf>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Manifest {
    schema: String,
    pub identity: String,
    pub cargo: bool,
    pub compiler: bool,
    times: BTreeMap<String, Stamp>,
}

pub(super) struct Snapshot {
    pub root: tempfile::TempDir,
    pub manifest: Manifest,
}

pub(super) struct Parcel<'a>(pub &'a Path);

impl Parcel<'_> {
    pub fn write(&self, plan: Plan) -> Result<Receipt, String> {
        pack::write(self.0, plan)
    }

    pub fn read(&self, identity: &str) -> Result<Option<Snapshot>, String> {
        unpack::read(self.0, identity)
    }
}
