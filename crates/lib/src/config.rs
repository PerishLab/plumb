use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum Error {
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Read { path, source } => write!(out, "cannot read {}: {source}", path.display()),
            Error::Parse { path, source } => {
                write!(out, "cannot parse {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Read { source, .. } => Some(source),
            Error::Parse { source, .. } => Some(source),
        }
    }
}

pub fn load<T: DeserializeOwned>(path: &Path) -> Result<T, Error> {
    let text = std::fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    toml::from_str(&text).map_err(|source| Error::Parse {
        path: path.to_path_buf(),
        source,
    })
}

pub fn discover(start: &Path, name: &str) -> Result<PathBuf, Vec<PathBuf>> {
    let mut searched = Vec::new();
    for dir in start.ancestors() {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Ok(candidate);
        }
        searched.push(candidate);
    }
    Err(searched)
}

pub fn rebase(path: &Path, base: &Path) -> PathBuf {
    if path.is_relative() {
        base.join(path)
    } else {
        path.to_path_buf()
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Listen {
    pub host: String,
    pub port: u16,
    pub prefix: String,
}

impl Default for Listen {
    fn default() -> Self {
        Listen {
            host: "127.0.0.1".to_string(),
            port: 3000,
            prefix: String::new(),
        }
    }
}

impl Listen {
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    #[default]
    Memory,
    File,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Store {
    pub kind: Kind,
    pub path: String,
}

impl Store {
    pub fn rebased(&self, base: &Path) -> PathBuf {
        rebase(Path::new(&self.path), base)
    }
}
