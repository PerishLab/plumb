use std::path::{Path, PathBuf};

#[cfg(feature = "depot")]
pub mod bucket;
pub mod depot;

pub use plumb_macro::{catalogue, resource};

pub const TMP: &str = ".tmp";

pub fn global(key: &str) -> Option<PathBuf> {
    if cfg!(windows) {
        return root("LOCALAPPDATA").map(|base| base.join(key));
    }
    home().map(|base| base.join(format!(".{key}")))
}

pub fn local(repository: &Path, key: &str) -> PathBuf {
    repository.join(format!(".{key}"))
}

pub fn tmp(repository: &Path, key: &str) -> PathBuf {
    repository.join(TMP).join(key)
}

pub fn home() -> Option<PathBuf> {
    root(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
}

fn root(key: &str) -> Option<PathBuf> {
    std::env::var_os(key)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}
