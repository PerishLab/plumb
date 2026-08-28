use serde::de::DeserializeOwned;
use std::fmt;
use std::path::{Path, PathBuf};

pub use plumb_macro::Cascade;

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
    Env {
        key: String,
        why: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Read { path, source } => write!(out, "cannot read {}: {source}", path.display()),
            Error::Parse { path, source } => {
                write!(out, "cannot parse {}: {source}", path.display())
            }
            Error::Env { key, why } => write!(out, "cannot parse {key}: {why}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Read { source, .. } => Some(source),
            Error::Parse { source, .. } => Some(source),
            Error::Env { .. } => None,
        }
    }
}

pub trait Env: Sized {
    fn read(value: &str) -> Result<Self, String>;
}

impl Env for String {
    fn read(value: &str) -> Result<Self, String> {
        Ok(value.to_string())
    }
}

impl Env for PathBuf {
    fn read(value: &str) -> Result<Self, String> {
        Ok(PathBuf::from(value))
    }
}

impl Env for bool {
    fn read(value: &str) -> Result<Self, String> {
        match value {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err("neither true nor false".to_string()),
        }
    }
}

macro_rules! parsed {
    ($($ty:ty),*) => {
        $(impl Env for $ty {
            fn read(value: &str) -> Result<Self, String> {
                value.parse().map_err(|error: std::num::ParseIntError| error.to_string())
            }
        })*
    };
}

parsed!(u16, u32, u64, usize, i32, i64);

impl<T: Env> Env for Option<T> {
    fn read(value: &str) -> Result<Self, String> {
        T::read(value).map(Some)
    }
}

pub struct Sniff<T>(pub std::marker::PhantomData<T>);

impl<T: Env> Sniff<T> {
    pub fn take(&self, held: Option<String>, key: &str) -> Result<Option<T>, Error> {
        let Some(value) = held else {
            return Ok(None);
        };
        let value = value.trim();
        if value.is_empty() {
            return Ok(None);
        }
        T::read(value).map(Some).map_err(|why| Error::Env {
            key: key.to_string(),
            why,
        })
    }
}

pub trait Opaque<T> {
    fn take(&self, held: Option<String>, key: &str) -> Result<Option<T>, Error>;
}

impl<T> Opaque<T> for &Sniff<T> {
    fn take(&self, _: Option<String>, _: &str) -> Result<Option<T>, Error> {
        Ok(None)
    }
}

pub trait Cascade: Default {
    type Partial: Default + DeserializeOwned;
    fn lookup(prefix: &str, get: &dyn Fn(&str) -> Option<String>) -> Result<Self::Partial, Error>;
    fn env(prefix: &str) -> Result<Self::Partial, Error> {
        Self::lookup(prefix, &|key| std::env::var(key).ok())
    }
    fn merge(self, over: Self::Partial) -> Self;
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

pub fn value(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|value| !value.is_empty())
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

pub fn home() -> Option<PathBuf> {
    crate::seat::home()
}

pub fn data(tool: &str) -> Option<PathBuf> {
    crate::seat::global(tool)
}

pub fn platform() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}
