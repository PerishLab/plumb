use super::super::{manager, proof};
use std::path::Path;
use std::process::{Command, Output};

pub fn run(root: &Path, channel: &str, version: &str, commit: &str) -> Result<String, String> {
    manager::intent(channel, version)?;
    proof::commit(commit)?;
    if channel != "stable" {
        return Err("only stable releases may create a Git tag".into());
    }
    let reference = format!("{version}^{{commit}}");
    let held = git(root, ["rev-parse", &reference])?;
    if held.status.success() {
        let actual = String::from_utf8_lossy(&held.stdout).trim().to_string();
        if actual == commit {
            return Ok(format!("stable tag {version} already names {commit}"));
        }
        return Err(format!("stable tag {version} names {actual}, not {commit}"));
    }
    for (key, value) in [
        ("user.name", "forgejo-actions"),
        ("user.email", "actions@perish.top"),
    ] {
        let output = git(root, ["config", key, value])?;
        success("configure Git tag author", output)?;
    }
    success(
        "create stable tag",
        git(root, ["tag", "-a", version, "-m", version, commit])?,
    )?;
    success("push stable tag", git(root, ["push", "origin", version])?)?;
    Ok(format!("tagged stable {version} at {commit}"))
}

fn git<const N: usize>(root: &Path, args: [&str; N]) -> Result<Output, String> {
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
