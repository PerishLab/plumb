use crate::command::release::{channel, proof};
use std::path::Path;
use std::process::{Command, Output};

pub struct Source<'a> {
    pub root: &'a Path,
    pub channel: &'a str,
    pub version: &'a str,
    pub commit: &'a str,
    pub reference: &'a str,
}

pub fn reference(held: &str) -> Result<String, String> {
    let version = held
        .strip_prefix("refs/tags/")
        .or_else(|| held.strip_prefix("refs/heads/release/"))
        .ok_or_else(|| {
            format!("release source must be an exact tag or a release line, got {held}")
        })?;
    channel::channel(version)?;
    Ok(version.to_string())
}

pub fn bind(input: Source<'_>) -> Result<String, String> {
    channel::intent(input.channel, input.version)?;
    proof::commit(input.commit)?;
    let reference = input.reference;
    success(
        "validate release source ref",
        git(input.root, ["check-ref-format", reference])?,
    )?;
    let expected = if input.channel == "stable" {
        format!("refs/heads/release/{}", input.version)
    } else {
        format!("refs/tags/{}", input.version)
    };
    if reference != expected {
        return Err(format!(
            "{} {} must originate from {expected}, got {reference}",
            input.channel, input.version
        ));
    }
    let actual = text(
        "resolve checked-out release commit",
        git(input.root, ["rev-parse", "HEAD^{commit}"])?,
    )?;
    if actual != input.commit {
        return Err(format!(
            "checked-out release commit is {actual}, not frozen commit {}",
            input.commit
        ));
    }
    Ok(format!(
        "bound {} {} to {reference} at {}",
        input.channel, input.version, input.commit
    ))
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

fn text(action: &str, output: Output) -> Result<String, String> {
    success(action, output.clone())?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
