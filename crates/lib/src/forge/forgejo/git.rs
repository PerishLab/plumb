use super::model::Remote;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub fn root() -> Result<PathBuf, String> {
    text(
        "locate repository",
        command(Path::new("."), ["rev-parse", "--show-toplevel"])?,
    )
    .map(PathBuf::from)
}

pub fn remote(root: &Path, repo: &str) -> Result<Remote, String> {
    let raw = text(
        "read origin",
        command(root, ["remote", "get-url", "origin"])?,
    )?;
    let mut remote = parse(&raw)?;
    if !repo.is_empty() {
        let parts = repo.split('/').collect::<Vec<_>>();
        if parts.len() != 2 || parts.iter().any(|held| held.is_empty()) {
            return Err(format!("--repo expects owner/name, got {repo}"));
        }
        remote.owner = parts[0].to_string();
        remote.repo = parts[1].to_string();
    }
    Ok(remote)
}

fn parse(raw: &str) -> Result<Remote, String> {
    let held = raw.trim();
    let (scheme, host, path) = if let Some((scheme, rest)) = held.split_once("://") {
        let (seat, path) = rest
            .split_once('/')
            .ok_or_else(|| format!("cannot derive owner/repo from remote url: {raw:?}"))?;
        let host = seat.rsplit_once('@').map_or(seat, |(_, host)| host);
        (scheme, host, path)
    } else {
        let (seat, path) = held
            .split_once(':')
            .ok_or_else(|| format!("unrecognized remote url: {raw:?}"))?;
        let host = seat.rsplit_once('@').map_or(seat, |(_, host)| host);
        ("https", host, path)
    };
    let parts = path
        .trim_start_matches('/')
        .trim_end_matches(".git")
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.len() < 2 {
        return Err(format!("cannot derive owner/repo from remote url: {raw:?}"));
    }
    if host == "github.com" || host.ends_with(".github.com") {
        return Err("origin must point to Forgejo".into());
    }
    Ok(Remote {
        scheme: if scheme == "http" { "http" } else { "https" }.to_string(),
        host: host.to_string(),
        owner: parts[parts.len() - 2].to_string(),
        repo: parts[parts.len() - 1].to_string(),
    })
}

pub fn tags(root: &Path, commit: &str) -> Result<Vec<String>, String> {
    let listed = text("list tags", command(root, ["tag", "--points-at", commit])?)?;
    Ok(listed
        .lines()
        .map(str::trim)
        .filter(|held| !held.is_empty())
        .map(str::to_string)
        .collect())
}

pub fn fetch(root: &Path) -> Result<(), String> {
    success(
        "fetch origin",
        command(root, ["fetch", "--prune", "origin"])?,
    )
}

fn command<const N: usize>(cwd: &Path, args: [&str; N]) -> Result<Output, String> {
    Command::new("git")
        .args(args)
        .current_dir(cwd)
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
