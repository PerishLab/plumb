use crate::dispatch::release::{channel, proof};
use std::path::Path;
use std::process::{Command, Output};

pub struct Source<'a> {
    pub root: &'a Path,
    pub channel: &'a str,
    pub version: &'a str,
    pub commit: &'a str,
    pub reference: &'a str,
}

pub struct Guard<'a> {
    pub api: &'a str,
    pub repository: &'a str,
    pub token: &'a str,
    pub commit: &'a str,
}

pub fn evidence(guard: Guard<'_>, contexts: &str) -> Result<String, String> {
    let wanted = named(contexts)?;
    if wanted.is_empty() {
        return Ok("no canonical guard evidence is required".to_string());
    }
    let harness = plumb::forgejo::harness()?;
    let deadline =
        std::time::Instant::now() + std::time::Duration::from_millis(harness.guard_timeout_ms);
    let url = format!(
        "{}/repos/{}/commits/{}/status",
        guard.api.trim_end_matches('/'),
        guard.repository,
        guard.commit
    );
    while std::time::Instant::now() < deadline {
        let states = poll(&url, guard.token, &wanted)?;
        if states.iter().all(|(_, state)| state == "success") {
            return Ok(format!("canonical guard proved {}", guard.commit));
        }
        if let Some((name, state)) = states
            .iter()
            .find(|(_, state)| state != "success" && state != "pending" && state != "missing")
        {
            return Err(format!(
                "canonical guard {name} is {state} on {}",
                guard.commit
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(harness.guard_pending_ms));
    }
    Err(format!(
        "canonical guard evidence did not settle on {} within {}s",
        guard.commit,
        harness.guard_timeout_ms / 1000
    ))
}

fn named(contexts: &str) -> Result<Vec<String>, String> {
    if contexts.trim().is_empty() {
        return Ok(Vec::new());
    }
    let parsed: serde_json::Value = serde_json::from_str(contexts)
        .map_err(|error| format!("guard contexts must be a JSON array: {error}"))?;
    let rows = parsed
        .as_array()
        .ok_or_else(|| "guard contexts must be a JSON array".to_string())?;
    rows.iter()
        .map(|row| {
            row.as_str()
                .filter(|held| !held.is_empty())
                .map(str::to_string)
                .ok_or_else(|| "each guard context must be a nonblank string".to_string())
        })
        .collect()
}

fn poll(url: &str, token: &str, wanted: &[String]) -> Result<Vec<(String, String)>, String> {
    let output = std::process::Command::new("curl")
        .args(["--fail-with-body", "--silent", "--show-error", "--location"])
        .arg("--header")
        .arg(format!("Authorization: token {token}"))
        .arg(url)
        .output()
        .map_err(|error| format!("cannot run curl: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cannot read commit status: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let record: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("cannot parse commit status: {error}"))?;
    let seen = record
        .get("statuses")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(wanted
        .iter()
        .map(|name| {
            let state = seen
                .iter()
                .find(|row| row.get("context").and_then(serde_json::Value::as_str) == Some(name))
                .and_then(|row| row.get("status").and_then(serde_json::Value::as_str))
                .unwrap_or("missing");
            (name.clone(), state.to_string())
        })
        .collect())
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

pub fn rejoin(root: &Path, version: &str, commit: &str, base_ref: &str) -> Result<String, String> {
    channel::intent("stable", version)?;
    proof::commit(commit)?;
    let base = text(
        "resolve rejoin base",
        git(root, ["rev-parse", &format!("{base_ref}^{{commit}}")])?,
    )?;
    if ancestor(root, commit, &base)? {
        Ok(format!(
            "stable {version} at {commit} is rejoined into {base_ref} at {base}"
        ))
    } else {
        Err(format!(
            "stable {version} at {commit} is not an ancestor of {base_ref} at {base}"
        ))
    }
}

pub fn ancestor(root: &Path, point: &str, base: &str) -> Result<bool, String> {
    let output = git(root, ["merge-base", "--is-ancestor", point, base])?;
    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(format!(
            "cannot inspect rejoin topology: {}",
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
