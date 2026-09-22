use std::path::{Path, PathBuf};
use std::process::Command;

pub fn root() -> Result<PathBuf, String> {
    git(
        Path::new("."),
        &["rev-parse", "--show-toplevel"],
        "locate repository",
    )
    .map(PathBuf::from)
}

pub fn fetch(root: &Path) -> Result<(), String> {
    git(root, &["fetch", "--prune", "origin"], "fetch origin").map(|_| ())
}

fn git(cwd: &Path, args: &[&str], deed: &str) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|error| format!("cannot run git: {error}"))?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }
    Err(format!(
        "{deed} failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    ))
}
