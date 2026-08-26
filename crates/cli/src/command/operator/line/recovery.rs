use super::super::value;
use std::path::Path;
use std::process::{Command, Output};

pub fn head(root: &Path, published: &str, projection: &str) -> Result<String, String> {
    let reference = format!("refs/heads/{projection}");
    success(
        "fetch the standing rejoin projection",
        command(root, &["fetch", "origin", &reference])?,
    )?;
    let head = read(
        "resolve the standing rejoin projection",
        command(root, &["rev-parse", "FETCH_HEAD^{commit}"])?,
    )?;
    value::commit(&head)?;
    let base = read(
        "resolve main for rejoin recovery",
        command(root, &["rev-parse", "origin/main^{commit}"])?,
    )?;
    let parents = read(
        "inspect rejoin recovery parents",
        command(root, &["rev-list", "--parents", "--max-count=1", &head])?,
    )?;
    if parents != format!("{head} {base} {published}") {
        return Err(format!(
            "open rejoin projection {projection} does not preserve main and the published stable commit"
        ));
    }
    let main = read(
        "resolve main tree for rejoin recovery",
        command(root, &["rev-parse", "origin/main^{tree}"])?,
    )?;
    let tree = read(
        "resolve projection tree for rejoin recovery",
        command(root, &["rev-parse", &format!("{head}^{{tree}}")])?,
    )?;
    if tree != main {
        return Err(format!(
            "open rejoin projection {projection} does not preserve main's tree"
        ));
    }
    Ok(head)
}

fn command(root: &Path, args: &[&str]) -> Result<Output, String> {
    Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| format!("cannot run git: {error}"))
}

fn success(action: &str, output: Output) -> Result<(), String> {
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "{action} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn read(action: &str, output: Output) -> Result<String, String> {
    success(action, output.clone())?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
