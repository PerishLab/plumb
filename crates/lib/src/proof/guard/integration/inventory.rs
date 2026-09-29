use super::{Refusal, Worktree};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn list(root: &Path) -> Result<Vec<Worktree>, Refusal> {
    let output = Command::new("git")
        .args([
            "-c",
            "core.quotePath=false",
            "worktree",
            "list",
            "--porcelain",
        ])
        .current_dir(root)
        .output()
        .map_err(|error| Refusal::new("integration.git", format!("cannot run git: {error}")))?;
    if !output.status.success() {
        return Err(Refusal::new(
            "integration.inventory",
            String::from_utf8_lossy(&output.stderr).trim(),
        ));
    }
    Ok(parse(&String::from_utf8_lossy(&output.stdout)))
}

fn parse(text: &str) -> Vec<Worktree> {
    let mut found = Vec::new();
    let mut current: Option<Worktree> = None;
    for line in text.lines().chain(std::iter::once("")) {
        if line.is_empty() {
            if let Some(held) = current.take() {
                found.push(held);
            }
            continue;
        }
        if let Some(path) = line.strip_prefix("worktree ") {
            if let Some(held) = current.take() {
                found.push(held);
            }
            current = Some(Worktree {
                path: canonical(path),
                branch: None,
                head: None,
                bare: false,
                detached: false,
                locked: false,
                prunable: false,
            });
            continue;
        }
        if let Some(held) = current.as_mut() {
            mark(held, line);
        }
    }
    found
}

fn mark(held: &mut Worktree, line: &str) {
    match line.split_once(' ') {
        Some(("HEAD", value)) => held.head = Some(value.to_string()),
        Some(("branch", value)) => {
            held.branch = value.strip_prefix("refs/heads/").map(str::to_string)
        }
        Some(("locked", _)) => held.locked = true,
        Some(("prunable", _)) => held.prunable = true,
        _ if line == "bare" => held.bare = true,
        _ if line == "detached" => held.detached = true,
        _ if line == "locked" => held.locked = true,
        _ if line == "prunable" => held.prunable = true,
        _ => {}
    }
}

fn canonical(path: &str) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path))
}
