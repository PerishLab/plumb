use plumb::forgejo::git::fetch;
use std::path::Path;
use std::process::Output;

pub(super) mod base;
mod evidence;
mod recovery;

pub fn validate(root: &Path, name: &str) -> Result<(), String> {
    fetch(root)?;
    let release = format!("origin/{name}");
    let version = name.strip_prefix("release/").unwrap_or(name);
    let base = base::resolve(root, &release, version)?;
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
    let source = Source {
        root,
        version,
        base: &base,
    };
    let commits = listed
        .split('\0')
        .filter_map(|record| record.trim_start().split_once('\u{1f}'));
    for (commit, body) in commits {
        if !sourced(&source, commit, body)? {
            return Err(format!(
                "{name} contains a commit without cherry-pick -x provenance: {commit}"
            ));
        }
    }
    Ok(())
}

struct Source<'a> {
    root: &'a Path,
    version: &'a str,
    base: &'a str,
}

fn sourced(source: &Source<'_>, commit: &str, body: &str) -> Result<bool, String> {
    let body = body.trim();
    let settled = super::datum::Seat(source.root).carried(commit, source.version)
        || super::version::prepared(super::version::Preparation {
            root: source.root,
            commit,
            base: source.base,
            version: source.version,
            body,
        });
    Ok(settled || refreshed(source, commit) || picked(source.root, commit, body)?)
}

fn refreshed(source: &Source<'_>, commit: &str) -> bool {
    let tree = format!("{commit}^{{tree}}");
    let parent = format!("{commit}^^{{tree}}");
    let trees = command(source.root, ["rev-parse", &tree, &parent])
        .ok()
        .and_then(|output| text("inspect proof carrier", output).ok());
    trees.is_some_and(|trees| {
        let mut trees = trees.lines();
        trees.next().is_some_and(|tree| trees.next() == Some(tree))
            && plumb::guard::commit(source.root, commit).is_ok()
    })
}

fn picked(root: &Path, commit: &str, body: &str) -> Result<bool, String> {
    let source = body
        .lines()
        .rev()
        .find_map(|line| line.strip_prefix("(cherry picked from commit "))
        .and_then(|line| line.strip_suffix(')'));
    let Some(source) = source.filter(|source| super::value::commit(source).is_ok()) else {
        return Ok(false);
    };
    let parent = command(root, ["rev-parse", &format!("{commit}^")])
        .and_then(|output| text("resolve picked parent", output));
    evidence::Seat(root).load(source)?;
    Ok(parent.is_ok_and(|parent| {
        recovery::Seat(root)
            .verify(&parent, commit, &[source.to_string()])
            .is_ok()
    }))
}

pub fn pick(seat: &Path, name: &str, commits: &[String]) -> Result<String, String> {
    plumb::depot::rules().map_err(|error| format!("pick configuration preflight: {error}"))?;
    controller(seat, name)?;
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
    let recovery = recovery::Seat(seat);
    recovery.clean()?;
    fetch(seat)?;
    recovery.unmarked(name)?;
    evidence::Seat(seat).available(commits)?;
    let head = text("resolve local head", command(seat, ["rev-parse", "HEAD"])?)?;
    let remote = text(
        "resolve remote head",
        command(seat, ["rev-parse", &format!("origin/{name}")])?,
    )?;
    if head == remote {
        if recovery.delivered(&head, commits)? {
            evidence::Seat(seat).retain(commits)?;
            return Ok(format!(
                "{} already picked onto {name}; nothing moved",
                commits.join(" ")
            ));
        }
        let picked = plumb::config::current("git")
            .args(["cherry-pick", "-x"])
            .args(commits)
            .current_dir(seat)
            .output()
            .map_err(|error| format!("cannot run git: {error}"))?;
        if let Err(error) = success("cherry-pick candidates", picked) {
            return Err(restore(seat, &head, error));
        }
    }
    let applied = text(
        "resolve applied pick",
        command(seat, ["rev-parse", "HEAD"])?,
    )?;
    recovery.verify(&remote, &applied, commits)?;
    sealed(seat)?;
    let prepared = text("resolve proved pick", command(seat, ["rev-parse", "HEAD"])?)?;
    recovery.unchanged(name, &remote)?;
    recovery.verify(&remote, &prepared, commits)?;
    plumb::guard::current(seat, &prepared)?;
    evidence::Seat(seat).retain(commits)?;
    success(
        "push release line",
        command(
            seat,
            [
                "push",
                &format!("--force-with-lease=refs/heads/{name}:{remote}"),
                "origin",
                &format!("{prepared}:refs/heads/{name}"),
            ],
        )?,
    )?;
    Ok(format!("picked {} onto {name}", commits.join(" ")))
}

fn controller(root: &Path, name: &str) -> Result<(), String> {
    let manifest = root.join("plumb.toml");
    if !manifest.is_file() && crate::shape::product::governance(root)?.is_none() {
        return Ok(());
    }
    let spec = if manifest.is_file() {
        crate::shape::release::Spec::read(&manifest)?
    } else {
        crate::shape::release::Spec::controller(root)?
    };
    if spec.product != "plumb" {
        return Ok(());
    }
    exact(name, plumb::version!("PLUMB"))
}

fn exact(name: &str, running: &str) -> Result<(), String> {
    let target = semver::Version::parse(name.trim_start_matches("release/v"))
        .map_err(|error| format!("cannot parse Plumb version line {name}: {error}"))?;
    let running = semver::Version::parse(running.trim_start_matches('v'))
        .map_err(|error| format!("cannot parse running Plumb version: {error}"))?;
    if (running.major, running.minor, running.patch) != (target.major, target.minor, target.patch) {
        return Err(format!(
            "{name} must be picked by Plumb v{}.{}.{}, not v{running}",
            target.major, target.minor, target.patch
        ));
    }
    Ok(())
}

fn restore(seat: &Path, head: &str, error: String) -> String {
    let _ = command(seat, ["cherry-pick", "--abort"]);
    match command(seat, ["reset", "--hard", head])
        .and_then(|done| success("restore release line", done))
    {
        Ok(()) => error,
        Err(recovery) => format!("{error}; {recovery}"),
    }
}

fn sealed(seat: &Path) -> Result<(), String> {
    let proof = crate::command::precommit::proof(seat)?;
    let body = text(
        "read picked commit message",
        command(seat, ["show", "-s", "--format=%B", "HEAD"])?,
    )?;
    let mut message = body
        .lines()
        .filter(|line| !line.starts_with(plumb::guard::TRAILER))
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end()
        .to_string();
    message.push_str("\n\n");
    message.push_str(plumb::guard::TRAILER);
    message.push(' ');
    message.push_str(&proof.encode()?);
    success(
        "seal picked commit",
        command(seat, ["commit", "--amend", "--no-verify", "-m", &message])?,
    )?;
    plumb::guard::current(seat, "HEAD").map(|_| ())
}

fn command<const N: usize>(cwd: &Path, args: [&str; N]) -> Result<Output, String> {
    plumb::config::current("git")
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
