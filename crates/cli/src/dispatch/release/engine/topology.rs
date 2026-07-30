use super::super::{manager, proof};
use std::path::Path;
use std::process::{Command, Output};

pub struct Source<'a> {
    pub root: &'a Path,
    pub channel: &'a str,
    pub version: &'a str,
    pub commit: &'a str,
    pub reference: &'a str,
}

pub fn source(input: Source<'_>) -> Result<String, String> {
    manager::intent(input.channel, input.version)?;
    proof::commit(input.commit)?;
    let reference = input.reference;
    if !reference.starts_with("refs/heads/") {
        return Err(format!(
            "release source must be a branch ref, got {reference}"
        ));
    }
    success(
        "validate release source ref",
        git(input.root, ["check-ref-format", reference])?,
    )?;
    if input.channel == "stable" {
        let expected = format!("refs/heads/release/{}", input.version);
        if reference != expected {
            return Err(format!(
                "stable {} must originate from {expected}, got {reference}",
                input.version
            ));
        }
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

pub fn packport(
    root: &Path,
    version: &str,
    commit: &str,
    base_ref: &str,
) -> Result<String, String> {
    manager::intent("stable", version)?;
    proof::commit(commit)?;
    let base = text(
        "resolve packport base",
        git(root, ["rev-parse", &format!("{base_ref}^{{commit}}")])?,
    )?;
    let output = git(root, ["merge-base", "--is-ancestor", commit, &base])?;
    match output.status.code() {
        Some(0) => Ok(format!(
            "stable {version} at {commit} is packported into {base_ref} at {base}"
        )),
        Some(1) => Err(format!(
            "stable {version} at {commit} is not an ancestor of {base_ref} at {base}"
        )),
        _ => Err(format!(
            "cannot inspect packport topology: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )),
    }
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
