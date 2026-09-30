use super::super::{Context, Evidence, Gate};
use crate::landing::Refusal;
use std::cell::Cell;
use std::path::Path;
use std::process::Command;

pub struct Repo(tempfile::TempDir);

impl Repo {
    pub fn new() -> Self {
        let repo = Self(tempfile::tempdir().expect("fixture"));
        git(repo.root(), &["init", "-q", "-b", "main"]);
        git(repo.root(), &["config", "user.name", "Native Test"]);
        git(
            repo.root(),
            &["config", "user.email", "native@example.invalid"],
        );
        git(repo.root(), &["config", "core.hooksPath", "unused-hooks"]);
        repo.commit("base", "base");
        git(
            repo.root(),
            &["update-ref", "refs/remotes/origin/main", "HEAD"],
        );
        git(repo.root(), &["checkout", "-q", "-b", "topic"]);
        repo.commit("work", "work");
        repo
    }

    pub fn root(&self) -> &Path {
        self.0.path()
    }

    pub fn commit(&self, path: &str, body: &str) {
        std::fs::write(self.root().join(path), body).expect("write");
        git(self.root(), &["add", "-A"]);
        git(self.root(), &["commit", "-q", "-m", "change"]);
    }

    pub fn merge(&self, candidate: &str, message: &str) -> String {
        let tree = git(
            self.root(),
            &["rev-parse", &format!("{candidate}^{{tree}}")],
        );
        let parent = git(self.root(), &["rev-parse", &format!("{candidate}^")]);
        git(
            self.root(),
            &["commit-tree", &tree, "-p", &parent, "-m", message],
        )
    }
}

pub struct Check {
    pub calls: Cell<usize>,
    pub event: &'static str,
}

impl Check {
    pub fn new(event: &'static str) -> Self {
        Self {
            calls: Cell::new(0),
            event,
        }
    }
}

impl Gate for Check {
    fn verify(&self, context: &Context<'_>) -> Result<Evidence, Refusal> {
        self.calls.set(self.calls.get() + 1);
        if self.event == "failure" {
            return Err(super::super::refuse("native", "gate failed"));
        }
        let mut evidence = Evidence {
            authority: "native.test/v1".to_string(),
            source: context.source.to_string(),
            tree: context.tree.to_string(),
            digest: "1".repeat(64),
        };
        match self.event {
            "source" => evidence.source = "0".repeat(40),
            "tree" => evidence.tree = "0".repeat(40),
            "authority" => evidence.authority.clear(),
            "digest" => evidence.digest = "invalid".to_string(),
            "base" => {
                git(
                    context.root,
                    &["update-ref", "refs/remotes/origin/main", context.source],
                );
            }
            "branch" => {
                git(context.root, &["checkout", "-q", "-b", "other"]);
            }
            "dirty" => {
                std::fs::write(context.root.join("dirty"), "changed").expect("dirty");
            }
            "head" => {
                git(
                    context.root,
                    &["commit", "--allow-empty", "-q", "-m", "moved"],
                );
            }
            _ => {}
        }
        Ok(evidence)
    }
}

pub fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .expect("git");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .to_string()
}
