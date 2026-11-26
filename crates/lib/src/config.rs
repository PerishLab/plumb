use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

pub use crate::runtime::cache::Cache;
pub use crate::runtime::environment::contract::contract;
pub use crate::runtime::environment::{Binding, Contract, Environment};
pub use crate::runtime::execution::{Execution, Search, Tool};
pub use plumb_macro::prefix;

pub fn environment(contract: &Contract) -> Result<Environment, String> {
    contract.capture(std::env::vars_os())
}

pub fn search(root: &Path) -> Result<Search, String> {
    Search::new(root, std::env::var_os("PATH"), std::env::var_os("PATHEXT"))
}

pub fn value(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|value| !value.is_empty())
}

pub fn migrated<'a>(
    primary: impl AsRef<str>,
    legacy: impl AsRef<str>,
    get: &'a dyn Fn(&str) -> Option<String>,
) -> impl Fn(&str) -> Option<String> + 'a {
    let primary = primary.as_ref().to_string();
    let legacy = legacy.as_ref().to_string();
    move |key: &str| {
        get(key).filter(|held| !held.trim().is_empty()).or_else(|| {
            key.strip_prefix(&primary)
                .filter(|rest| rest.starts_with('_'))
                .and_then(|rest| get(&format!("{legacy}{rest}")))
                .filter(|held| !held.trim().is_empty())
        })
    }
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

pub fn binary() -> Option<PathBuf> {
    std::env::current_exe().ok()
}

pub fn current(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    let Some(parent) = binary().and_then(|path| path.parent().map(Path::to_path_buf)) else {
        return command;
    };
    let mut paths = vec![parent];
    if let Some(path) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&path));
    }
    if let Ok(path) = std::env::join_paths(paths) {
        command.env("PATH", path);
    }
    command
}

pub fn detached(program: impl AsRef<OsStr>) -> Command {
    radicula_git::detached(program)
}

#[cfg(any(feature = "vendor", feature = "windtun"))]
pub mod github {
    use runseal::github::{Client, Error};
    use std::path::Path;

    pub fn client() -> Result<Client, String> {
        let value = crate::config::value;
        if let Some(path) = value("GITHUB_TOKEN_FILE") {
            return Client::open(Path::new(&path)).map_err(diagnostic);
        }
        let token = value("GH_TOKEN")
            .or_else(|| value("GITHUB_TOKEN"))
            .ok_or("select a profile carrying a GitHub token")?;
        Client::token(&token).map_err(diagnostic)
    }

    pub fn diagnostic(error: Error) -> String {
        format!(
            "{error}; HTTP status {:?}; request {:?}; retry-after {:?}",
            error.evidence.status, error.evidence.request, error.evidence.wait
        )
    }
}
