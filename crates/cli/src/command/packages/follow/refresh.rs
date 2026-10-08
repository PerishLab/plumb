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
    let output = work::git(root, &["merge-tree", "--write-tree", &head, &base])?;
    let tree = output.lines().next().ok_or("Auto base merge has no tree")?;
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
