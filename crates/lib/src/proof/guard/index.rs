use super::{Descriptor, staged, store, transit};
use std::path::{Path, PathBuf};

pub fn tree(root: &Path) -> Result<String, String> {
    let (source, temporary) = locate(root, false)?;
    Snapshot { source, temporary }.tree(root)
}

pub fn bind(
    root: &Path,
    message: &Path,
    prove: impl FnOnce(&Path) -> Result<Descriptor, String>,
) -> Result<Descriptor, String> {
    let (tree, temporary) = Snapshot::commit(root)?;
    if temporary {
        staged(root, &tree).map_err(|error| {
            format!("no matching pre-commit guard proof for commit tree {tree}: {error}")
        })?;
    }
    let proof = prove(root)?;
    let (current, _) = Snapshot::commit(root)?;
    if proof.tree != tree || proof.tree != current {
        return Err(format!(
            "commit index tree changed during guard attachment (started at {tree}, now {current}, proof seals {})",
            proof.tree
        ));
    }
    if staged(root, &current)? != proof {
        return Err(format!(
            "staged guard proof changed for commit tree {current}"
        ));
    }
    transit::write(message, proof)
}

fn locate(root: &Path, committing: bool) -> Result<(PathBuf, bool), String> {
    let ordinary = PathBuf::from(store::git(
        root,
        &["rev-parse", "--path-format=absolute", "--git-path", "index"],
        "locate staged index",
    )?);
    let Some(supplied) = std::env::var_os("GIT_INDEX_FILE").filter(|path| !path.is_empty()) else {
        return Ok((ordinary, false));
    };
    let supplied = std::path::absolute(supplied)
        .map_err(|error| format!("cannot resolve Git commit index: {error}"))?;
    let parent = ordinary
        .parent()
        .ok_or("staged index has no parent")?
        .canonicalize()
        .map_err(|error| format!("cannot resolve staged index directory: {error}"))?;
    let owned = supplied
        .parent()
        .and_then(|directory| directory.canonicalize().ok())
        .is_some_and(|directory| directory == parent);
    if !owned {
        if committing {
            return Err(
                "cannot attach guard proof for an index outside this worktree's Git directory"
                    .into(),
            );
        }
        return Ok((ordinary, false));
    }
    if supplied.file_name() == ordinary.file_name() {
        return Ok((ordinary, false));
    }
    let metadata = std::fs::symlink_metadata(&supplied)
        .map_err(|error| format!("cannot read Git commit index: {error}"))?;
    if !metadata.is_file() {
        return Err("Git commit index must be a regular file".into());
    }
    Ok((supplied, true))
}

struct Snapshot {
    source: PathBuf,
    temporary: bool,
}

impl Snapshot {
    fn commit(root: &Path) -> Result<(String, bool), String> {
        let (source, temporary) = locate(root, true)?;
        Ok((Self { source, temporary }.tree(root)?, temporary))
    }

    fn tree(&self, root: &Path) -> Result<String, String> {
        let bytes = match std::fs::read(&self.source) {
            Ok(bytes) => bytes,
            Err(error) if !self.temporary && error.kind() == std::io::ErrorKind::NotFound => {
                return store::git(root, &["mktree"], "read empty staged tree");
            }
            Err(error) => return Err(format!("cannot read staged index: {error}")),
        };
        let parent = self.source.parent().ok_or("staged index has no parent")?;
        let index = tempfile::Builder::new()
            .prefix("plumb-index-")
            .tempfile_in(parent)
            .map_err(|error| format!("cannot reserve staged index snapshot: {error}"))?;
        std::fs::write(index.path(), bytes)
            .map_err(|error| format!("cannot copy staged index: {error}"))?;
        let output = crate::config::detached("git")
            .arg("-C")
            .arg(root)
            .arg("write-tree")
            .env("GIT_INDEX_FILE", index.path())
            .output()
            .map_err(|error| format!("cannot read staged tree snapshot: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "cannot read staged tree snapshot: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().into())
    }
}
