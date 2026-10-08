pub(crate) mod follow;
use plumb::packages::{Plan, Resolution};
use std::path::{Path, PathBuf};

pub(crate) fn resolve(root: &Path, context: &str) -> Result<Resolution, String> {
    let plan = Plan::read(root, context)?;
    plan.apply(root)?;
    let changes = super::guard::precommit::tree::git(
        root,
        &["diff", "--name-only", "HEAD"],
        "inspect dependency delta",
    )?;
    let paths = changes.lines().collect::<Vec<_>>();
    for path in &paths {
        if !plumb::packages::allowed(path) {
            return Err(format!(
                "dependency resolution changed forbidden path {path}"
            ));
        }
    }
    if !paths.is_empty() {
        let mut args = vec!["add", "--"];
        args.extend(paths);
        super::guard::precommit::tree::git(root, &args, "stage resolved dependencies")?;
    }
    plumb::packages::locked(root, context)
}

pub(crate) fn lift(root: PathBuf, json: bool) -> i32 {
    match apply(&root) {
        Ok(resolution) => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string(&resolution).expect("resolution encodes")
                );
            } else {
                println!(
                    "first-party packages resolved: {}",
                    resolution.packages.len()
                );
            }
            0
        }
        Err(error) => {
            eprintln!("plumb lift: {error}");
            1
        }
    }
}

fn apply(root: &Path) -> Result<Resolution, String> {
    let status = super::guard::precommit::tree::git(
        root,
        &["status", "--porcelain", "--untracked-files=all"],
        "inspect lift worktree",
    )?;
    if !status.is_empty() {
        return Err("lift requires a clean worktree".into());
    }
    let branch = super::guard::precommit::tree::git(
        root,
        &["branch", "--show-current"],
        "inspect lift branch",
    )?;
    if matches!(branch.as_str(), "main" | "master") {
        return Err("lift requires a mutable topic worktree".into());
    }
    let tree = plumb::guard::tree(root)?;
    let index = super::guard::precommit::tree::Index::new(root, &tree)?;
    let resolution = resolve(&index.root, "lift")?;
    let changes = super::guard::precommit::tree::git(
        &index.root,
        &["diff", "--name-only", "HEAD"],
        "inspect lift delta",
    )?;
    let fresh = super::guard::precommit::tree::git(
        root,
        &["status", "--porcelain", "--untracked-files=all"],
        "revalidate lift worktree",
    )?;
    if !fresh.is_empty() || plumb::guard::tree(root)? != tree {
        return Err("lift worktree changed while resolving dependencies".into());
    }
    for path in changes.lines() {
        let bytes = std::fs::read(index.root.join(path)).map_err(|error| error.to_string())?;
        std::fs::write(root.join(path), bytes).map_err(|error| error.to_string())?;
    }
    Ok(resolution)
}
