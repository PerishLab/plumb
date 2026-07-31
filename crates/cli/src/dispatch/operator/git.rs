use super::api::Remote;
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

pub fn fetch(root: &Path) -> Result<(), String> {
    success(
        "fetch origin",
        command(root, ["fetch", "--prune", "origin"])?,
    )
}

pub fn validate(root: &Path, name: &str) -> Result<(), String> {
    fetch(root)?;
    let release = format!("origin/{name}");
    let base = text(
        "resolve release base",
        command(root, ["merge-base", "origin/main", &release])?,
    )?;
    let range = format!("{base}..{release}");
    let merges = text(
        "inspect release merges",
        command(root, ["rev-list", "--min-parents=2", &range])?,
    )?;
    if !merges.is_empty() {
        return Err(format!("{name} must remain linear"));
    }
    let bodies = text(
        "inspect release provenance",
        command(root, ["log", "--format=%B%x00", &range])?,
    )?;
    let invalid = bodies
        .split('\0')
        .map(str::trim)
        .find(|body| !body.is_empty() && !provenance(body));
    if invalid.is_some() {
        return Err(format!(
            "{name} contains a commit without cherry-pick -x provenance"
        ));
    }
    Ok(())
}

fn provenance(body: &str) -> bool {
    body.split("(cherry picked from commit ")
        .skip(1)
        .filter_map(|tail| tail.split_once(')'))
        .any(|(commit, _)| super::value::commit(commit).is_ok())
}

pub fn pick(seat: &Path, name: &str, commit: &str) -> Result<String, String> {
    let current = text(
        "read current branch",
        command(seat, ["branch", "--show-current"])?,
    )?;
    if current != name {
        return Err(format!(
            "pick must run from {name}, got {}",
            if current.is_empty() {
                "detached HEAD"
            } else {
                &current
            }
        ));
    }
    if !text("inspect worktree", command(seat, ["status", "--short"])?)?.is_empty() {
        return Err("pick requires a clean worktree".into());
    }
    fetch(seat)?;
    let head = text("resolve local head", command(seat, ["rev-parse", "HEAD"])?)?;
    let remote = text(
        "resolve remote head",
        command(seat, ["rev-parse", &format!("origin/{name}")])?,
    )?;
    if head != remote {
        return Err(format!("{name} must equal origin/{name} before pick"));
    }
    success(
        "cherry-pick candidate",
        command(seat, ["cherry-pick", "-x", commit])?,
    )?;
    success(
        "push release line",
        command(seat, ["push", "origin", &format!("HEAD:refs/heads/{name}")])?,
    )?;
    Ok(format!("picked {commit} onto {name}"))
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
