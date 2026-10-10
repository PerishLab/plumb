use super::{State, scope, work};
use std::path::Path;

pub(super) fn prepare(state: &State) -> Result<Option<String>, String> {
    let root = &state.worktree;
    clean(root)?;
    let head = work::git(root, &["rev-parse", "HEAD"])?;
    let base = work::git(root, &["rev-parse", "origin/main"])?;
    let common = work::git(root, &["merge-base", &head, &base])?;
    if common == base {
        return Ok(None);
    }
    let known = state.pushed.as_ref() == Some(&head)
        || state.candidate.as_ref() == Some(&head)
        || state.plan.as_ref().is_some_and(|plan| plan.source == head);
    if !known {
        return Err("Auto local head moved outside recorded recovery state".into());
    }
    scope::check(root, &common, &head)?;
    let tree = match merged(root, &head, &base)? {
        Some(tree) => tree,
        None => work::git(root, &["rev-parse", &format!("{base}^{{tree}}")])?,
    };
    let tree = tree.as_str();
    if !matches!(tree.len(), 40 | 64) || !tree.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("Auto base merge did not produce an exact clean tree".into());
    }
    let commit = work::git(
        root,
        &[
            "commit-tree",
            tree,
            "-p",
            &head,
            "-p",
            &base,
            "-m",
            &format!("Resume follow on current main\n\nRefs #{}.", state.issue),
        ],
    )?;
    scope::check(root, &base, &commit)?;
    Ok(Some(commit))
}

fn merged(root: &Path, head: &str, base: &str) -> Result<Option<String>, String> {
    let output = plumb::config::detached("git")
        .arg("-C")
        .arg(root)
        .args(["merge-tree", "--write-tree", head, base])
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|error| error.to_string())?;
    match output.status.code() {
        Some(0) => String::from_utf8(output.stdout)
            .map_err(|error| error.to_string())?
            .lines()
            .next()
            .map(|tree| Some(tree.to_string()))
            .ok_or_else(|| "Auto base merge has no tree".into()),
        Some(1) => Ok(None),
        _ => Err(format!(
            "Auto base merge failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )),
    }
}

pub(super) fn apply(root: &Path, commit: &str) -> Result<(), String> {
    clean(root)?;
    let parents = work::git(root, &["show", "-s", "--format=%P", commit])?;
    let held = parents.split_whitespace().collect::<Vec<_>>();
    if held.len() != 2
        || work::git(root, &["rev-parse", "HEAD"])? != held[0]
        || work::git(root, &["rev-parse", "origin/main"])? != held[1]
    {
        return Err("Auto source or main moved during base refresh; reread before retry".into());
    }
    work::git(root, &["merge", "--ff-only", "--no-edit", commit])?;
    if work::git(root, &["rev-parse", "HEAD"])? != commit {
        return Err("Auto base refresh head disagrees".into());
    }
    clean(root)
}

fn clean(root: &Path) -> Result<(), String> {
    if work::git(root, &["status", "--porcelain", "--untracked-files=all"])?.is_empty() {
        Ok(())
    } else {
        Err(
            "Auto worktree has preserved local payload; commit or recover it before resuming"
                .into(),
        )
    }
}
