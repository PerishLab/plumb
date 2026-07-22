use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};

pub fn load<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    toml::from_str(&text).map_err(|error| format!("cannot parse {}: {error}", path.display()))
}

pub fn discover(name: &str) -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|error| format!("cannot read cwd: {error}"))?;
    for dir in cwd.ancestors() {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(format!("no {name} found from {} upward", cwd.display()))
}
