use super::{Release, Value};
use std::path::{Path, PathBuf};

pub const FORMAT: u32 = 2;
pub const LEAF: &str = "manifest.toml";
pub const POINTER: &str = "latest.json";

pub fn local(root: &Path, release: &Release, timestamp: &str) -> Result<PathBuf, String> {
    release.validate()?;
    Value(timestamp).component("snapshot timestamp")?;
    Ok(root.join("v2").join(&release.version).join(timestamp))
}
