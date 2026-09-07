use plumb::config::Environment;
use std::path::{Path, PathBuf};

pub(super) fn cargo(root: &Path) -> Result<Environment, String> {
    let environment = crate::execution::environment("cargo")?;
    let root = root
        .canonicalize()
        .map_err(|error| format!("cannot resolve Cargo root: {error}"))?;
    crate::execution::inspect("cargo", &root, &environment)?;
    let home = plumb::config::value("PLUMB_HOME")
        .map(PathBuf::from)
        .or_else(|| plumb::config::data("plumb"))
        .ok_or_else(|| "Cargo execution has no managed home".to_string())?;
    crate::execution::inspect("cargo", &home.join("tmp/guard"), &environment)?;
    Ok(environment)
}
