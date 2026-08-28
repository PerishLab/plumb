use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    #[serde(rename = "type")]
    pub kind: String,
    pub source: String,
}

impl Source {
    pub fn parse(text: &str) -> Result<Self, String> {
        let held: Self =
            serde_json::from_str(text).map_err(|error| format!("cannot parse --reuse: {error}"))?;
        held.validate()?;
        Ok(held)
    }

    fn validate(&self) -> Result<(), String> {
        match (self.kind.as_str(), self.source.as_str()) {
            ("none", "") => Ok(()),
            ("workload" | "url", source) if source.starts_with("https://") => Ok(()),
            ("none" | "workload" | "url", _) => {
                Err("--reuse source does not match its type".into())
            }
            _ => Err("--reuse type must be none, workload, or url".into()),
        }
    }
}

pub fn fetch(kind: &str, source: &str) -> Result<Vec<u8>, String> {
    let response = Command::new("curl")
        .args([
            "--fail-with-body",
            "--silent",
            "--show-error",
            "--location",
            "--retry",
            "3",
            source,
        ])
        .output()
        .map_err(|error| format!("cannot fetch reusable {kind} workload {source}: {error}"))?;
    if response.status.success() {
        Ok(response.stdout)
    } else {
        Err(format!("cannot fetch reusable {kind} workload {source}"))
    }
}

pub struct Scope {
    path: PathBuf,
    original: Vec<u8>,
    restored: bool,
}

impl Scope {
    pub fn read(path: &Path) -> Result<Self, String> {
        let original = std::fs::read(path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        Ok(Self {
            path: path.to_path_buf(),
            original,
            restored: false,
        })
    }

    pub fn restore(mut self) -> Result<(), String> {
        std::fs::write(&self.path, &self.original)
            .map_err(|error| format!("cannot restore {}: {error}", self.path.display()))?;
        self.restored = true;
        Ok(())
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        if !self.restored {
            let _ = std::fs::write(&self.path, &self.original);
        }
    }
}
