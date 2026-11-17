use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
pub(super) struct Invocation<'a> {
    pub schema: &'static str,
    pub cwd: &'a Path,
    pub arguments: &'a [String],
    pub directory: &'a Path,
    pub registry: &'a Path,
    pub tools: BTreeMap<String, plumb::config::Tool>,
    pub timeout: u64,
}

#[derive(Serialize)]
pub(super) struct Report {
    pub schema: &'static str,
    pub code: Option<i32>,
    pub signal: Option<i32>,
    pub cause: String,
    pub process: bool,
    pub directory: bool,
    pub cleanup: bool,
    pub exit: i32,
    pub error: Option<String>,
}

impl Report {
    pub fn failed(error: String) -> Self {
        Self {
            schema: "plumb.cache-run/v1",
            code: None,
            signal: None,
            cause: "setup".into(),
            process: true,
            directory: false,
            cleanup: false,
            exit: 125,
            error: Some(error),
        }
    }
}

pub(super) fn write(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let parent = path.parent().ok_or("output has no parent")?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|error| error.to_string())?;
    serde_json::to_writer_pretty(&mut file, value).map_err(|error| error.to_string())?;
    writeln!(file).map_err(|error| error.to_string())?;
    file.persist_noclobber(path)
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[derive(Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Record {
    pub schema: String,
    pub keeper: String,
    pub directory: PathBuf,
    pub device: u64,
    pub inode: u64,
}

pub(super) fn read(path: &Path) -> Result<Record, String> {
    let file = std::fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if !file.is_file() {
        return Err(format!(
            "{} is not a regular ownership record",
            path.display()
        ));
    }
    serde_json::from_slice(&std::fs::read(path).map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())
}
