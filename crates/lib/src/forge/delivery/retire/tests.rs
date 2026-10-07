use super::{Delivered, Kept, Pushed, retired};
use crate::guard::{Action, Descriptor, TRAILER};
use std::path::{Path, PathBuf};
use std::process::Command;

const PROJECTION: &str = "land/topic";
const TOPIC: &str = "topic";

struct Repo {
    fixture: tempfile::TempDir,
    base: String,
    source: String,
    candidate: String,
}

impl Repo {
    fn new() -> Self {
        let fixture = tempfile::tempdir().expect("fixture");
        let root = fixture.path().join("work");
        std::fs::create_dir(&root).expect("work");
        git(fixture.path(), &["init", "-q", "--bare", "origin.git"]);
        git(&root, &["init", "-q", "-b", "main"]);
        git(&root, &["config", "user.name", "Retire Test"]);
        git(&root, &["config", "user.email", "retire@example.invalid"]);
        let origin = fixture.path().join("origin.git");
        git(
            &root,
            &["remote", "add", "origin", origin.to_str().expect("utf8")],
        );
        std::fs::write(root.join("README.md"), "base\n").expect("write");
        git(&root, &["add", "-A"]);
        git(&root, &["commit", "-q", "-m", "base"]);
        let base = git(&root, &["rev-parse", "HEAD"]);
        git(&root, &["push", "-q", "origin", "main"]);
        git(&root, &["checkout", "-q", "-b", TOPIC]);
        std::fs::write(root.join("topic.md"), "work\n").expect("write");
        git(&root, &["add", "-A"]);
        git(&root, &["commit", "-q", "-m", "work"]);
        let source = git(&root, &["rev-parse", "HEAD"]);
        let tree = git(&root, &["write-tree"]);
        let token = proof(&root, &tree).encode().expect("token");
        let message = format!("Add the topic\n\n{TRAILER} {token}\n");
        let candidate = git(&root, &["commit-tree", &tree, "-p", &base, "-m", &message]);
        git(&root, &["push", "-q", "origin", TOPIC]);
        git(
            &root,
            &[
                "push",
                "-q",
                "origin",
                &format!("{candidate}:refs/heads/{PROJECTION}"),
            ],
        );
        Self {
            fixture,
            base,
            source,
            candidate,
        }
    }

    fn root(&self) -> PathBuf {
        self.fixture.path().join("work")
    }

    fn merge(&self, parent: &str) -> String {
        let root = self.root();
        let tree = git(
            &root,
            &["rev-parse", &format!("{}^{{tree}}", self.candidate)],
        );
        let message = git(&root, &["log", "-1", "--format=%B", &self.candidate]);
        git(&root, &["commit-tree", &tree, "-p", parent, "-m", &message])
    }

    fn pushed(&self) -> [Pushed; 2] {
        [
            Pushed {
                branch: PROJECTION.into(),
                head: self.candidate.clone(),
            },
            Pushed {
                branch: TOPIC.into(),
                head: self.source.clone(),
            },
        ]
    }

    fn origin(&self, branch: &str) -> String {
        git(
            &self.root(),
            &[
                "ls-remote",
                "--heads",
                "origin",
                &format!("refs/heads/{branch}"),
            ],
        )
    }

    fn local(&self, branch: &str) -> bool {
        Command::new("git")
            .arg("-C")
            .arg(self.root())
            .args([
                "show-ref",
                "--verify",
                "--quiet",
                &format!("refs/heads/{branch}"),
            ])
            .status()
            .expect("git")
            .success()
    }
}

fn delivered<'a>(root: &'a Path, candidate: &'a str, head: &'a str) -> Delivered<'a> {
    Delivered {
        root,
        remote: "origin",
        candidate,
        head,
    }
}

#[test]
fn deleted() {
    let repo = Repo::new();
    let root = repo.root();
    let head = repo.merge(&repo.base);
    let (landed, retired) =
        retired(&delivered(&root, &repo.candidate, &head), &repo.pushed()).expect("retired");
    assert_eq!(landed.head, head);
    assert_eq!(retired.deleted, [PROJECTION, TOPIC]);
    assert!(retired.kept.is_empty(), "{:?}", retired.kept);
    for branch in [PROJECTION, TOPIC] {
        assert!(repo.origin(branch).is_empty(), "{branch} stays on origin");
        assert!(!repo.local(branch), "{branch} stays local");
    }
    assert_eq!(git(&root, &["rev-parse", "HEAD"]), repo.source);
}

#[test]
fn unproven() {
    let repo = Repo::new();
    let root = repo.root();
    let head = repo.merge(&repo.source);
    retired(&delivered(&root, &repo.candidate, &head), &repo.pushed())
        .expect_err("a merge on another parent is not landed");
    for branch in [PROJECTION, TOPIC] {
        assert!(!repo.origin(branch).is_empty(), "{branch} left origin");
    }
    assert!(repo.local(TOPIC));
}

#[test]
fn moved() {
    let repo = Repo::new();
    let root = repo.root();
    git(
        &root,
        &[
            "push",
            "-q",
            "--force",
            "origin",
            &format!("{}:refs/heads/{PROJECTION}", repo.base),
        ],
    );
    git(&root, &["commit", "-q", "--allow-empty", "-m", "later"]);
    let later = git(&root, &["rev-parse", "HEAD"]);
    let head = repo.merge(&repo.base);
    let (_, retired) =
        retired(&delivered(&root, &repo.candidate, &head), &repo.pushed()).expect("retired");
    assert!(retired.deleted.is_empty(), "{:?}", retired.deleted);
    assert_eq!(
        retired.kept,
        [
            Kept {
                branch: PROJECTION.into(),
                seat: "origin".into(),
                reason: format!("it holds {}, not the pushed {}", repo.base, repo.candidate),
            },
            Kept {
                branch: TOPIC.into(),
                seat: "local".into(),
                reason: format!("it moved to {later} after the push"),
            },
        ]
    );
    assert!(!repo.origin(PROJECTION).is_empty());
    assert!(
        repo.origin(TOPIC).is_empty(),
        "the unmoved origin topic is deleted"
    );
    assert!(repo.local(TOPIC));
}

#[test]
fn elsewhere() {
    let repo = Repo::new();
    let root = repo.root();
    git(&root, &["checkout", "-q", "--detach"]);
    let other = repo.fixture.path().join("other");
    git(
        &root,
        &[
            "worktree",
            "add",
            "-q",
            other.to_str().expect("utf8"),
            TOPIC,
        ],
    );
    let head = repo.merge(&repo.base);
    let (_, retired) =
        retired(&delivered(&root, &repo.candidate, &head), &repo.pushed()).expect("retired");
    assert_eq!(retired.deleted, [PROJECTION]);
    assert_eq!(retired.kept.len(), 1, "{:?}", retired.kept);
    assert_eq!(retired.kept[0].seat, "local");
    assert!(retired.kept[0].reason.starts_with("it is checked out at "));
    assert!(repo.local(TOPIC));
    assert_eq!(git(&other, &["rev-parse", "--abbrev-ref", "HEAD"]), TOPIC);
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
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}
