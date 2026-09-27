use super::{Request, SCHEMA};
use crate::guard::{Action, Authority, Descriptor, Expected, Verified};
use std::path::Path;
use std::process::{Command, Output};

const COMMIT: &str = "1111111111111111111111111111111111111111";
const DEPOT: &str = "2222222222222222222222222222222222222222222222222222222222222222";

struct Repo {
    fixture: tempfile::TempDir,
    root: std::path::PathBuf,
    proof: Descriptor,
}

struct Git<'a>(&'a Path);

impl Git<'_> {
    fn run(&self, args: &[&str]) -> Output {
        let output = Command::new("git")
            .arg("-C")
            .arg(self.0)
            .args(args)
            .output()
            .expect("git");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    fn write(&self, name: &str, value: &str) {
        std::fs::write(self.0.join(name), value).expect("write");
    }

    fn commit(&self, message: &str) {
        self.run(&["add", "-A"]);
        self.run(&["commit", "-q", "-m", message]);
    }

    fn text(&self, args: &[&str]) -> String {
        String::from_utf8_lossy(&self.run(args).stdout)
            .trim()
            .to_string()
    }
}

impl Repo {
    fn new() -> Self {
        let fixture = tempfile::tempdir().expect("fixture");
        let origin = fixture.path().join("origin.git");
        let root = fixture.path().join("work");
        std::fs::create_dir_all(&root).expect("work");
        Git(fixture.path()).run(&["init", "-q", "--bare", "origin.git"]);
        let git = Git(&root);
        git.run(&["init", "-q", "-b", "main"]);
        git.run(&["config", "user.name", "Landing Test"]);
        git.run(&["config", "user.email", "landing@example.invalid"]);
        git.run(&["remote", "add", "origin", origin.to_str().expect("utf8")]);
        git.write("README.md", "base\n");
        git.commit("base");
        git.run(&["push", "-q", "-u", "origin", "main"]);
        git.run(&["checkout", "-q", "-b", "topic"]);
        git.write("topic.md", "work\n");
        git.commit("Add exact landing");
        git.run(&[
            "remote",
            "set-url",
            "origin",
            "https://github.com/PerishLab/probe.git",
        ]);
        let tree = git.text(&["rev-parse", "HEAD^{tree}"]);
        let proof = proof(tree, "guard/test");
        let token = proof.encode().expect("proof");
        let message = format!("Add exact landing\n\n{} {token}", crate::guard::TRAILER);
        git.run(&["commit", "-q", "--amend", "-m", &message]);
        Self {
            fixture,
            root,
            proof,
        }
    }

    fn request<'a>(&'a self, body: &'a str) -> Request<'a> {
        Request {
            root: &self.root,
            base: "main",
            title: "Exact landing",
            body,
        }
    }

    fn verified(&self) -> Verified {
        verified(&self.root, self.proof.clone())
    }
}

fn proof(tree: String, name: &str) -> Descriptor {
    let mut proof = Descriptor {
        schema: crate::guard::SCHEMA.into(),
        repository: "PerishLab/probe".into(),
        tree,
        plumb: format!("v0.0.0@{COMMIT}"),
        depot: DEPOT.into(),
        platform: crate::config::platform(),
        actions: vec![Action {
            name: name.into(),
            input: "3".repeat(64),
            world: "4".repeat(64),
        }],
        digest: String::new(),
    };
    proof.digest = proof.seal().expect("seal");
    proof
}

fn verified(seat: &Path, proof: Descriptor) -> Verified {
    let expected = Expected::held(&proof);
    Authority::fixture(format!("v0.0.0@{COMMIT}"), DEPOT)
        .judge(seat, proof, &expected)
        .expect("verified")
}

#[test]
fn exact() {
    let repo = Repo::new();
    assert!(repo.fixture.path().is_dir());
    let inspected = repo
        .request("Refs PerishLab/probe#1")
        .inspect()
        .expect("inspect");
    let prepared = inspected.prepare(repo.verified()).expect("prepare");
    let ready = repo
        .request("Refs PerishLab/probe#1")
        .revalidate(&prepared, repo.verified())
        .expect("revalidate");
    assert_eq!(ready.preparation(), &prepared);
    assert_eq!(SCHEMA, "plumb.landing/v1");
}

#[test]
fn narrative() {
    let repo = Repo::new();
    let prepared = repo
        .request("Refs PerishLab/probe#1")
        .inspect()
        .expect("inspect")
        .prepare(repo.verified())
        .expect("prepare");
    let refusal = repo
        .request("Closes PerishLab/probe#1")
        .revalidate(&prepared, repo.verified())
        .expect_err("changed narrative");
    assert_eq!(refusal.kind, "stale");
}

#[test]
fn evidence() {
    let repo = Repo::new();
    let changed = proof(repo.proof.tree.clone(), "guard/changed");
    let refusal = repo
        .request("Refs PerishLab/probe#1")
        .inspect()
        .expect("inspect")
        .prepare(verified(&repo.root, changed))
        .expect_err("different verified proof");
    assert_eq!(refusal.kind, "guard");
}

#[test]
fn candidate() {
    let repo = Repo::new();
    let mut prepared = repo
        .request("Refs PerishLab/probe#1")
        .inspect()
        .expect("inspect")
        .prepare(repo.verified())
        .expect("prepare");
    prepared.candidate = "5".repeat(40);
    let refusal = repo
        .request("Refs PerishLab/probe#1")
        .revalidate(&prepared, repo.verified())
        .expect_err("changed candidate");
    assert_eq!(refusal.kind, "stale");
}

#[test]
fn source() {
    let repo = Repo::new();
    let prepared = repo
        .request("Refs PerishLab/probe#1")
        .inspect()
        .expect("inspect")
        .prepare(repo.verified())
        .expect("prepare");
    let git = Git(&repo.root);
    git.write("later.md", "changed\n");
    git.commit("Change the source");
    let refusal = repo
        .request("Refs PerishLab/probe#1")
        .revalidate(&prepared, repo.verified())
        .expect_err("changed source");
    assert_eq!(refusal.kind, "guard");
}

#[test]
fn base() {
    let repo = Repo::new();
    let prepared = repo
        .request("Refs PerishLab/probe#1")
        .inspect()
        .expect("inspect")
        .prepare(repo.verified())
        .expect("prepare");
    Git(&repo.root).run(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
    let refusal = repo
        .request("Refs PerishLab/probe#1")
        .revalidate(&prepared, repo.verified())
        .expect_err("changed base");
    assert_eq!(refusal.kind, "empty");
}
