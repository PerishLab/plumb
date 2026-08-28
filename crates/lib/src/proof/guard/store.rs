use super::{Descriptor, TRAILER, hash, home};
use sha2::{Digest as _, Sha256};
use std::path::{Path, PathBuf};
use std::process::Output;

pub(super) struct Seat {
    root: PathBuf,
    pub repository: String,
}

impl Seat {
    pub(super) fn new(root: &Path) -> Result<Self, String> {
        let remote = git(root, &["remote", "get-url", "origin"], "read origin")?;
        let held = remote.trim_end_matches('/').trim_end_matches(".git");
        let path = held
            .rsplit_once(':')
            .filter(|(left, _)| !left.contains('/'))
            .map_or(held, |(_, path)| path);
        let mut parts = path.split('/').rev();
        let repo = parts.next().unwrap_or_default();
        let owner = parts.next().unwrap_or_default();
        if owner.is_empty() || repo.is_empty() {
            return Err(format!(
                "cannot derive repository identity from origin {remote:?}"
            ));
        }
        Ok(Self {
            root: root.to_path_buf(),
            repository: format!("{owner}/{repo}"),
        })
    }

    pub(super) fn matches(&self, proof: &Descriptor) -> Result<(), String> {
        if proof.repository == self.repository {
            Ok(())
        } else {
            Err(format!(
                "guard proof names {}, not {}",
                proof.repository, self.repository
            ))
        }
    }

    pub(super) fn pending(&self, tree: &str) -> Result<PathBuf, String> {
        hash(tree, "tree")?;
        let name = format!("{:x}", Sha256::digest(self.repository.as_bytes()));
        Ok(home()?
            .join("proof")
            .join("guard")
            .join(name)
            .join(format!("{tree}.json")))
    }

    pub(super) fn staged(&self, tree: &str) -> Result<Descriptor, String> {
        let path = self.pending(tree)?;
        let bytes = std::fs::read(&path).map_err(|error| {
            format!("cannot read staged guard proof {}: {error}", path.display())
        })?;
        let proof: Descriptor = serde_json::from_slice(&bytes).map_err(|error| {
            format!(
                "cannot parse staged guard proof {}: {error}",
                path.display()
            )
        })?;
        proof.validate()?;
        proof.current(&self.root)?;
        if proof.tree != tree {
            return Err(format!(
                "staged guard proof seals {}, not {tree}",
                proof.tree
            ));
        }
        Ok(proof)
    }

    pub(super) fn committed(&self, commit: &str) -> Result<Descriptor, String> {
        let message = git(
            &self.root,
            &["show", "-s", "--format=%B", commit],
            "read commit message",
        )?;
        let tokens = message
            .lines()
            .filter_map(|line| line.strip_prefix(TRAILER).map(str::trim))
            .collect::<Vec<_>>();
        let [token] = tokens.as_slice() else {
            return Err(format!(
                "commit {commit} must carry exactly one {TRAILER} trailer"
            ));
        };
        let proof = Descriptor::decode(token)?;
        let tree = git(
            &self.root,
            &["rev-parse", &format!("{commit}^{{tree}}")],
            "read commit tree",
        )?;
        if proof.tree != tree {
            return Err(format!(
                "commit {commit} has tree {tree}, but its guard proof seals {}",
                proof.tree
            ));
        }
        self.matches(&proof)?;
        Ok(proof)
    }
}

pub(super) fn git(root: &Path, args: &[&str], action: &str) -> Result<String, String> {
    let output = crate::config::detached("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|error| format!("cannot run git to {action}: {error}"))?;
    success(output, action)
}

fn success(output: Output, action: &str) -> Result<String, String> {
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(format!(
            "cannot {action}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}
