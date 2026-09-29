use super::{Squash, landed};
use crate::guard::{Action, Descriptor, TRAILER};
use std::path::Path;
use std::process::Command;

struct Repo {
    fixture: tempfile::TempDir,
    base: String,
    candidate: String,
}

impl Repo {
    fn new() -> Self {
        let fixture = tempfile::tempdir().expect("fixture");
        let root = fixture.path();
        git(root, &["init", "-q", "-b", "main"]);
        git(root, &["config", "user.name", "Squash Test"]);
        git(root, &["config", "user.email", "squash@example.invalid"]);
        git(
            root,
            &[
                "remote",
                "add",
                "origin",
                "https://github.com/PerishLab/probe.git",
            ],
        );
        std::fs::write(root.join("README.md"), "base\n").expect("write");
        git(root, &["add", "-A"]);
        git(root, &["commit", "-q", "-m", "base"]);
        let base = git(root, &["rev-parse", "HEAD"]);
        std::fs::write(root.join("topic.md"), "work\n").expect("write");
        git(root, &["add", "-A"]);
        let tree = git(root, &["write-tree"]);
        let token = proof(root, &tree).encode().expect("token");
        let message = format!("Add the topic\n\nWhy it lands.\n\n{TRAILER} {token}\n");
        let candidate = git(root, &["commit-tree", &tree, "-p", &base, "-m", &message]);
        Self {
            fixture,
            base,
            candidate,
        }
    }

    fn root(&self) -> &Path {
        self.fixture.path()
    }

    fn squash(&self) -> Squash {
        Squash::read(self.root(), &self.candidate).expect("squash")
    }

    fn merge(&self, tree: &str, parent: &str, message: &str) -> String {
        git(
            self.root(),
            &["commit-tree", tree, "-p", parent, "-m", message],
        )
    }

    fn tree(&self, commit: &str) -> String {
        git(self.root(), &["rev-parse", &format!("{commit}^{{tree}}")])
    }

    fn message(&self, squash: &Squash) -> String {
        format!("{}\n\n{}", squash.subject, squash.body)
    }
}

fn proof(root: &Path, tree: &str) -> Descriptor {
    crate::depot::carry(&[("rules/fixture.toml", "")]);
    let action = Action {
        name: "guard/test".into(),
        input: "3".repeat(64),
        world: "4".repeat(64),
    };
    Descriptor::new(root, tree.to_string(), vec![action]).expect("proof")
}

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .expect("git");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn contract() {
    let repo = Repo::new();
    let squash = repo.squash();
    assert_eq!(squash.subject, "Add the topic");
    assert!(
        squash.body.starts_with("Why it lands.\n\n"),
        "{}",
        squash.body
    );
    assert!(squash.body.contains(TRAILER), "{}", squash.body);
    let candidate = repo.candidate.as_str();
    assert_eq!(
        squash.arguments("PerishLab/probe", 79),
        [
            "pr",
            "merge",
            "79",
            "-R",
            "PerishLab/probe",
            "--squash",
            "--match-head-commit",
            candidate,
            "--subject",
            "Add the topic",
            "--body",
            squash.body.as_str(),
        ]
    );
}

#[test]
fn passes() {
    let repo = Repo::new();
    let squash = repo.squash();
    let tree = repo.tree(&repo.candidate);
    let head = repo.merge(&tree, &repo.base, &repo.message(&squash));
    let held = landed(repo.root(), &repo.candidate, &head).expect("landed");
    assert_eq!((held.head, held.parent, held.tree), (head, repo.base, tree));
}

#[test]
fn moved() {
    let repo = Repo::new();
    let squash = repo.squash();
    let other = repo.merge(&repo.tree(&repo.base), &repo.base, "raced ahead");
    let head = repo.merge(&repo.tree(&repo.candidate), &other, &repo.message(&squash));
    let refusal = landed(repo.root(), &repo.candidate, &head).expect_err("moved");
    assert_eq!(refusal.kind, "moved", "{}", refusal.message);
    assert!(
        refusal.message.contains("no Guard proved")
            && refusal.message.contains(&other)
            && refusal.message.contains(&repo.base),
        "{}",
        refusal.message
    );
}

#[test]
fn differs() {
    let repo = Repo::new();
    let squash = repo.squash();
    let head = repo.merge(&repo.tree(&repo.base), &repo.base, &repo.message(&squash));
    let refusal = landed(repo.root(), &repo.candidate, &head).expect_err("tree");
    assert_eq!(refusal.kind, "tree", "{}", refusal.message);
}

#[test]
fn lost() {
    let repo = Repo::new();
    let head = repo.merge(&repo.tree(&repo.candidate), &repo.base, "Add the topic");
    let refusal = landed(repo.root(), &repo.candidate, &head).expect_err("proof");
    assert_eq!(refusal.kind, "proof", "{}", refusal.message);
}
