use super::context::Context;
use super::inputs::{Input, relative};
use super::stamp::Stamp;
use super::target::{Seal, Target};
use super::{MARKER, SCHEMA};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Hint {
    pub schema: String,
    pub ready: bool,
    pub source: PathBuf,
    pub world: String,
    pub image: String,
    pub stamp: Stamp,
    pub target: Option<Seal>,
    pub files: BTreeMap<String, Input>,
}

impl Hint {
    pub fn read(bucket: &Path) -> Result<Option<Self>, String> {
        let path = bucket.join(MARKER);
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.to_string()),
        };
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err("Guard material snapshot is not a regular owned file".into());
        }
        if metadata.len() > 8 * 1024 * 1024 {
            return Ok(None);
        }
        let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
        Ok(serde_json::from_slice(&bytes).ok())
    }

    pub fn write(&self, bucket: &Path) -> Result<(), String> {
        let path = bucket.join(MARKER);
        if let Ok(metadata) = std::fs::symlink_metadata(&path)
            && (!metadata.is_file() || metadata.file_type().is_symlink())
        {
            return Err("Guard material snapshot is not a regular owned file".into());
        }
        let bytes = serde_json::to_vec(self).map_err(|error| error.to_string())?;
        if bytes.len() > 8 * 1024 * 1024 {
            return Err("Guard material source snapshot exceeds 8 MiB".into());
        }
        plumb::replace::write(&path, &bytes).map_err(|error| error.to_string())
    }

    pub fn valid(&self, actual: &Seal, now: Stamp) -> bool {
        if !self.ready || self.schema != SCHEMA {
            return false;
        }
        if self.stamp.time().is_err() || self.stamp > now {
            return false;
        }
        let Some(sealed) = &self.target else {
            return false;
        };
        if sealed.sha != actual.sha || sealed.files != actual.files || actual.latest > self.stamp {
            return false;
        }
        self.files
            .iter()
            .all(|(name, input)| relative(name) && input.valid(self.stamp))
    }

    pub fn ready(bucket: &Path, context: &Context) -> Result<bool, String> {
        let Some(hint) = Self::read(bucket)? else {
            return Ok(false);
        };
        if hint.image != context.image || !Target(bucket).supported()? {
            return Ok(false);
        }
        let actual = Target(bucket).seal()?;
        if !hint.valid(&actual, Stamp::now()?) {
            return Ok(false);
        }
        Ok(super::super::graph::profiles(bucket)?
            .iter()
            .any(|profile| !profile.units.is_empty()))
    }
}
