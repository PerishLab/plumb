use plumb::integration::{Expectation, Relation, advance, inspect};
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

struct Fixture {
    _temporary: TempDir,
    seed: PathBuf,
    checkout: PathBuf,
    initial: String,
    target: String,
}

impl Fixture {
    fn new() -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let remote = temporary.path().join("remote.git");
        let seed = temporary.path().join("seed");
        let checkout = temporary.path().join("checkout");
        git(
            temporary.path(),
            &["init", "--bare", remote.to_str().unwrap()],
        );
        git(
            temporary.path(),
            &["init", "-b", "main", seed.to_str().unwrap()],
        );
        identity(&seed);
        std::fs::write(seed.join("story"), "initial\n").unwrap();
        git(&seed, &["add", "story"]);
        git(&seed, &["commit", "-m", "initial"]);
        git(
            &seed,
            &["remote", "add", "origin", remote.to_str().unwrap()],
        );
        git(&seed, &["push", "-u", "origin", "main"]);
        git(&remote, &["symbolic-ref", "HEAD", "refs/heads/main"]);
        git(
            temporary.path(),
            &[
                "clone",
                remote.to_str().unwrap(),
                checkout.to_str().unwrap(),
            ],
        );
        identity(&checkout);
        let initial = revision(&checkout, "HEAD");
        std::fs::write(seed.join("story"), "target\n").unwrap();
        git(&seed, &["commit", "-am", "target"]);
        git(&seed, &["push", "origin", "main"]);
        let target = revision(&seed, "HEAD");
        git(&checkout, &["fetch", "origin", "main"]);
        Self {
            _temporary: temporary,
            seed,
            checkout,
            initial,
            target,
        }
    }

    fn expected(&self) -> Expectation {
        Expectation::new("main", "origin/main", &self.target)
    }

    fn record(&self, message: &str) -> String {
        std::fs::write(self.checkout.join("local"), format!("{message}\n")).unwrap();
        git(&self.checkout, &["add", "local"]);
        git(&self.checkout, &["commit", "-m", message]);
        revision(&self.checkout, "HEAD")
    }
}

#[test]
fn relations() {
    let fixture = Fixture::new();
    let held = inspect(&fixture.checkout, &fixture.expected()).unwrap();
    assert_eq!(held.relation, Relation::Behind);
    assert_eq!(held.checkout.head, fixture.initial);
    assert_eq!(
        held.checkout.tracked.as_deref(),
        Some(fixture.target.as_str())
    );
    assert!(held.checkout.clean);

    git(&fixture.checkout, &["merge", "--ff-only", &fixture.target]);
    assert_eq!(
        inspect(&fixture.checkout, &fixture.expected())
            .unwrap()
            .relation,
        Relation::Equal
    );
    fixture.record("ahead");
    assert_eq!(
        inspect(&fixture.checkout, &fixture.expected())
            .unwrap()
            .relation,
        Relation::Ahead
    );
}

#[test]
fn inventory() {
    let fixture = Fixture::new();
    let sibling = fixture._temporary.path().join("sibling");
    git(
        &fixture.checkout,
        &["worktree", "add", "-b", "topic", sibling.to_str().unwrap()],
    );
    let held = inspect(&fixture.checkout, &fixture.expected()).unwrap();
    assert_eq!(held.worktrees.len(), 2);
    assert!(held.worktrees.iter().any(|worktree| {
        worktree.path.canonicalize().unwrap() == fixture.checkout.canonicalize().unwrap()
            && worktree.branch.as_deref() == Some("main")
    }));
    assert!(
        held.worktrees
            .iter()
            .any(|worktree| worktree.path.canonicalize().unwrap()
                == sibling.canonicalize().unwrap()
                && worktree.branch.as_deref() == Some("topic"))
    );
}

#[test]
fn idempotent() {
    let fixture = Fixture::new();
    let advanced = advance(&fixture.checkout, &fixture.expected(), &fixture.initial).unwrap();
    assert_eq!(advanced.relation, Relation::Equal);
    assert_eq!(advanced.checkout.head, fixture.target);
    assert_eq!(advanced.checkout.tree, advanced.tree);
    let repeated = advance(&fixture.checkout, &fixture.expected(), &fixture.target).unwrap();
    assert_eq!(repeated.checkout.head, fixture.target);
}

#[test]
fn dirty() {
    let fixture = Fixture::new();
    std::fs::write(fixture.checkout.join("untracked"), "held\n").unwrap();
    let dirty = advance(&fixture.checkout, &fixture.expected(), &fixture.initial).unwrap_err();
    assert_eq!(dirty.code, "integration.dirty");
    std::fs::remove_file(fixture.checkout.join("untracked")).unwrap();

    git(&fixture.checkout, &["checkout", "--detach"]);
    let detached = advance(&fixture.checkout, &fixture.expected(), &fixture.initial).unwrap_err();
    assert_eq!(detached.code, "integration.branch");
    assert_eq!(detached.inspection.unwrap().checkout.branch, None);

    git(&fixture.checkout, &["checkout", "-b", "other"]);
    let other = advance(&fixture.checkout, &fixture.expected(), &fixture.initial).unwrap_err();
    assert_eq!(other.code, "integration.branch");
}

#[test]
fn upstream() {
    let fixture = Fixture::new();
    git(
        &fixture.checkout,
        &["update-ref", "refs/remotes/origin/other", &fixture.target],
    );
    git(
        &fixture.checkout,
        &["config", "branch.main.merge", "refs/heads/other"],
    );
    let wrong = advance(&fixture.checkout, &fixture.expected(), &fixture.initial).unwrap_err();
    assert_eq!(wrong.code, "integration.upstream");

    git(
        &fixture.checkout,
        &["config", "branch.main.merge", "refs/heads/main"],
    );
    git(
        &fixture.checkout,
        &["update-ref", "-d", "refs/remotes/origin/main"],
    );
    let absent = inspect(&fixture.checkout, &fixture.expected()).unwrap();
    assert_eq!(absent.checkout.tracked, None);
    let refused = advance(&fixture.checkout, &fixture.expected(), &fixture.initial).unwrap_err();
    assert_eq!(refused.code, "integration.tracking");
}

#[test]
fn histories() {
    let ahead = Fixture::new();
    git(&ahead.checkout, &["merge", "--ff-only", &ahead.target]);
    let local = ahead.record("ahead");
    let refused = advance(&ahead.checkout, &ahead.expected(), &local).unwrap_err();
    assert_eq!(refused.code, "integration.ahead");

    let diverged = Fixture::new();
    let local = diverged.record("diverged");
    let refused = advance(&diverged.checkout, &diverged.expected(), &local).unwrap_err();
    assert_eq!(refused.code, "integration.diverged");
}

#[test]
fn changed() {
    let fixture = Fixture::new();
    git(&fixture.checkout, &["merge", "--ff-only", &fixture.target]);
    let refused = advance(&fixture.checkout, &fixture.expected(), &fixture.initial).unwrap_err();
    assert_eq!(refused.code, "integration.changed");
    assert_eq!(refused.inspection.unwrap().checkout.head, fixture.target);
}

#[test]
fn exact() {
    let fixture = Fixture::new();
    let abbreviated = &fixture.target[..12];
    let refused = inspect(
        &fixture.checkout,
        &Expectation::new("main", "origin/main", abbreviated),
    )
    .unwrap_err();
    assert_eq!(refused.code, "integration.target");

    let missing = inspect(
        &fixture.checkout,
        &Expectation::new(
            "main",
            "origin/main",
            "0000000000000000000000000000000000000000",
        ),
    )
    .unwrap_err();
    assert_eq!(missing.code, "integration.target");
}

#[test]
fn offline() {
    let fixture = Fixture::new();
    std::fs::write(fixture.seed.join("later"), "later\n").unwrap();
    git(&fixture.seed, &["add", "later"]);
    git(&fixture.seed, &["commit", "-m", "later"]);
    git(&fixture.seed, &["push", "origin", "main"]);
    let before = revision(&fixture.checkout, "refs/remotes/origin/main");
    let _ = inspect(&fixture.checkout, &fixture.expected()).unwrap();
    assert_eq!(
        revision(&fixture.checkout, "refs/remotes/origin/main"),
        before
    );
}

fn identity(root: &Path) {
    git(root, &["config", "user.name", "Plumb Test"]);
    git(root, &["config", "user.email", "plumb@example.invalid"]);
}

fn revision(root: &Path, reference: &str) -> String {
    let output = Command::new("git")
        .args(["rev-parse", "--verify", reference])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
}
