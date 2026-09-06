use plumb::land::{Request, plan, run};
use std::path::Path;
use std::process::{Command, Output};

struct Repo {
    #[allow(dead_code)]
    fixture: tempfile::TempDir,
    work: std::path::PathBuf,
}

impl Repo {
    fn new() -> Self {
        let fixture = tempfile::tempdir().expect("fixture");
        let upstream = fixture.path().join("origin.git");
        let work = fixture.path().join("work");
        std::fs::create_dir_all(&work).expect("work");
        git(fixture.path(), &["init", "-q", "--bare", "origin.git"]);
        git(&work, &["init", "-q", "-b", "main"]);
        git(&work, &["config", "user.name", "Plumb Test"]);
        git(&work, &["config", "user.email", "plumb@example.invalid"]);
        git(
            &work,
            &["remote", "add", "origin", upstream.to_str().expect("utf8")],
        );
        let held = Self { fixture, work };
        held.write("README.md", "base\n");
        held.commit("base: seed the line");
        git(held.root(), &["push", "-q", "-u", "origin", "main"]);
        git(
            held.root(),
            &[
                "remote",
                "set-url",
                "origin",
                "ssh://git@git.test/PerishLab/fixture.git",
            ],
        );
        held
    }

    fn root(&self) -> &Path {
        &self.work
    }

    fn write(&self, path: &str, text: &str) {
        let path = self.root().join(path);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, text).expect("write");
    }

    fn commit(&self, message: &str) {
        git(self.root(), &["add", "-A"]);
        git(self.root(), &["commit", "-q", "-m", message]);
    }

    fn branch(&self, name: &str) {
        git(self.root(), &["checkout", "-q", "-b", name]);
    }

    fn request(&self) -> Request<'_> {
        Request {
            root: self.root(),
            base: "main",
            title: "",
            body: "",
            watch: false,
        }
    }
}

fn git(root: &Path, args: &[&str]) -> Output {
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
    output
}

#[test]
fn onbase() {
    let repo = Repo::new();
    let refusal = plan(repo.request()).expect_err("main is not landable");
    assert_eq!(refusal.kind, "onbase");
}

#[test]
fn release() {
    let repo = Repo::new();
    repo.branch("release/v1.2.3");
    let refusal = plan(repo.request()).expect_err("release lines are not landable");
    assert_eq!(refusal.kind, "release");
}

#[test]
fn dirty() {
    let repo = Repo::new();
    repo.branch("topic");
    repo.write("README.md", "dirty\n");
    let refusal = plan(repo.request()).expect_err("a dirty tree is not landable");
    assert_eq!(refusal.kind, "dirty");
}

#[test]
fn empty() {
    let repo = Repo::new();
    repo.branch("topic");
    let refusal = plan(repo.request()).expect_err("an empty topic is not landable");
    assert_eq!(refusal.kind, "empty");
}

#[test]
fn detached() {
    let repo = Repo::new();
    git(repo.root(), &["checkout", "-q", "--detach"]);
    let refusal = plan(repo.request()).expect_err("detached HEAD is not landable");
    assert_eq!(refusal.kind, "detached");
}

#[test]
fn noupstream() {
    let repo = Repo::new();
    repo.branch("topic");
    repo.write("topic.md", "work\n");
    repo.commit("topic: add work");
    let request = Request {
        base: "nonexistent",
        ..repo.request()
    };
    let refusal = plan(request).expect_err("a missing base is not landable");
    assert_eq!(refusal.kind, "noupstream");
}

#[test]
fn planned() {
    let repo = Repo::new();
    repo.branch("topic");
    repo.write("topic.md", "work\n");
    repo.commit("topic: add work");
    let before = String::from_utf8_lossy(&git(repo.root(), &["rev-parse", "HEAD"]).stdout)
        .trim()
        .to_string();

    let plan = plan(repo.request()).expect("a clean topic plans");
    assert_eq!(plan.branch, "topic");
    assert_eq!(plan.projection, "land/topic");
    assert_eq!(plan.base, "main");
    assert!(
        plan.steps
            .iter()
            .any(|step| step.contains("delete_branch_after_merge=false")),
        "the plan must state that no branch is deleted: {:?}",
        plan.steps
    );
    assert!(
        plan.steps
            .iter()
            .any(|step| step.contains("guard / guard (pull_request)")),
        "the proof status must be visible before merge: {:?}",
        plan.steps
    );
    assert!(
        !plan.steps.iter().any(|step| step.contains("branch -D")),
        "no plan step may delete a branch: {:?}",
        plan.steps
    );

    let after = String::from_utf8_lossy(&git(repo.root(), &["rev-parse", "HEAD"]).stdout)
        .trim()
        .to_string();
    assert_eq!(before, after, "planning must not move HEAD");
    assert!(
        !repo.root().join(".git/refs/heads/land").exists(),
        "planning must not create the projection"
    );
}

#[test]
fn refusal() {
    let repo = Repo::new();
    let refusal = run(repo.request()).expect_err("main is not landable");
    assert_eq!(refusal.kind, "onbase");
    assert!(refusal.message.contains("topic branch"));
    assert!(!refusal.to_string().is_empty());
}
