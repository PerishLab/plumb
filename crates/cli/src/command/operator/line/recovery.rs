use super::super::value;
use std::path::Path;
use std::process::{Command, Output};

pub enum Head {
    Current(String),
    Stale,
}

pub fn head(root: &Path, published: &str, projection: &str) -> Result<Head, String> {
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
    let lineage = read(
        "inspect rejoin recovery parents",
        command(root, &["rev-list", "--parents", "--max-count=1", &head])?,
    )?;
    let parents = lineage.split_whitespace().collect::<Vec<_>>();
    if parents.len() != 3 || parents[0] != head || parents[2] != published {
        return Err(format!(
            "open rejoin projection {projection} does not preserve a main commit and the published stable commit"
        ));
    }
    let first = parents[1];
    let expected = format!("{first}^{{tree}}");
    let expected = read(
        "resolve the projected main tree for rejoin recovery",
        command(root, &["rev-parse", &expected])?,
    )?;
    let tree = read(
        "resolve projection tree for rejoin recovery",
        command(root, &["rev-parse", &format!("{head}^{{tree}}")])?,
    )?;
    if tree != expected {
        return Err(format!(
            "open rejoin projection {projection} does not preserve its main parent's tree"
        ));
    }
    if first == base {
        return Ok(Head::Current(head));
    }
    success(
        "prove the projected main remains in current main",
        command(root, &["merge-base", "--is-ancestor", first, "origin/main"])?,
    )?;
    Ok(Head::Stale)
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
