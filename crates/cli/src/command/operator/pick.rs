use plumb::forgejo::git::fetch;
use std::path::Path;
use std::process::{Command, Output};

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
    let listed = text(
        "inspect release provenance",
        command(root, ["log", "--format=%H%x1f%B%x00", &range])?,
    )?;
    let version = name.strip_prefix("release/").unwrap_or(name);
    let source = Source {
        root,
        version,
        base: &base,
    };
    let invalid = listed
        .split('\0')
        .filter_map(|record| record.trim_start().split_once('\u{1f}'))
        .find(|(commit, body)| !sourced(&source, commit, body));
    if invalid.is_some() {
        return Err(format!(
            "{name} contains a commit without cherry-pick -x provenance"
        ));
    }
    Ok(())
}

struct Source<'a> {
    root: &'a Path,
    version: &'a str,
    base: &'a str,
}

fn sourced(source: &Source<'_>, commit: &str, body: &str) -> bool {
    let body = body.trim();
    if body.is_empty() {
        return true;
    }
    provenance(body)
        || super::datum::Seat(source.root).carried(commit, source.version)
        || super::version::prepared(super::version::Preparation {
            root: source.root,
            commit,
            base: source.base,
            version: source.version,
            body,
        })
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
