use super::{Standing, contained, report, standing};
use std::path::{Path, PathBuf};
use std::process::Command;

struct Line {
    #[allow(dead_code)]
    fixture: tempfile::TempDir,
    work: PathBuf,
}

impl Line {
    fn new() -> Self {
        let fixture = tempfile::tempdir().expect("fixture");
        let work = fixture.path().join("work");
        std::fs::create_dir_all(&work).expect("work");
        git(fixture.path(), &["init", "-q", "--bare", "origin.git"]);
        git(&work, &["init", "-q", "-b", "main"]);
        git(&work, &["config", "user.name", "Rejoin Test"]);
        git(&work, &["config", "user.email", "rejoin@example.invalid"]);
        let origin = fixture.path().join("origin.git");
        let origin = origin.to_str().expect("utf8").replace('\\', "/");
        git(&work, &["remote", "add", "origin", &origin]);
        let held = Self { fixture, work };
        held.commit("README.md", "base\n", "base");
        held
    }

    fn root(&self) -> &Path {
        &self.work
    }

    fn commit(&self, path: &str, text: &str, message: &str) -> String {
        std::fs::write(self.work.join(path), text).expect("write");
        git(&self.work, &["add", "-A"]);
        git(&self.work, &["commit", "-q", "-m", message]);
        git(&self.work, &["rev-parse", "HEAD"])
    }

    fn cut(&self, text: &str) -> String {
        git(
            &self.work,
            &["checkout", "-q", "-b", "release/v1.0.0", "main"],
        );
        let commit = self.commit("fix.txt", text, "fix on the release line");
        git(&self.work, &["checkout", "-q", "main"]);
        commit
    }

    fn stable(&self, branch: &str) {
        git(&self.work, &["tag", "-a", "v1.0.0", branch, "-m", "v1.0.0"]);
        git(&self.work, &["push", "-q", "origin", "main", "v1.0.0"]);
    }
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
fn ancestor() {
    let line = Line::new();
    let main = git(line.root(), &["rev-parse", "HEAD"]);
    assert_eq!(
        standing(line.root(), &main, &main).expect("standing"),
        Standing::Ancestor
    );
}

#[test]
fn squashed() {
    let line = Line::new();
    let released = line.cut("fixed\n");
    let main = line.commit("fix.txt", "fixed\n", "Squash the release fix into main");
    assert_eq!(
        standing(line.root(), &released, &main).expect("standing"),
        Standing::Contained
    );
    assert!(contained(line.root(), &released, &main).expect("contained"));
}

#[test]
fn lacking() {
    let line = Line::new();
    let released = line.cut("fixed\n");
    let main = git(line.root(), &["rev-parse", "HEAD"]);
    assert_eq!(
        standing(line.root(), &released, &main).expect("standing"),
        Standing::Lacking(vec!["fix.txt".to_string()])
    );
    assert!(!contained(line.root(), &released, &main).expect("contained"));
}

#[test]
fn conflicting() {
    let line = Line::new();
    let released = line.cut("fixed on the line\n");
    let main = line.commit("fix.txt", "fixed otherwise\n", "Fix main otherwise");
    assert_eq!(
        standing(line.root(), &released, &main).expect("standing"),
        Standing::Conflicting(vec!["fix.txt".to_string()])
    );
}

#[test]
fn reports() {
    let line = Line::new();
    git(line.root(), &["push", "-q", "origin", "main"]);
    assert_eq!(report(line.root()).expect("report").state, "unmarked");

    let released = line.cut("fixed\n");
    line.stable("release/v1.0.0");
    let owed = report(line.root()).expect("report");
    assert_eq!(
        (owed.state, owed.commit.as_deref(), owed.lacking.as_slice()),
        (
            "owed",
            Some(released.as_str()),
            ["fix.txt".to_string()].as_slice()
        )
    );
    let before = git(line.root(), &["ls-remote", "origin"]);

    line.commit("fix.txt", "fixed\n", "Squash the release fix into main");
    git(line.root(), &["push", "-q", "origin", "main"]);
    let home = report(line.root()).expect("report");
    assert_eq!((home.state, home.lacking.len()), ("home", 0));
    let after = git(line.root(), &["ls-remote", "origin"]);
    assert_eq!(
        before
            .lines()
            .filter(|held| !held.contains("refs/heads/main"))
            .collect::<Vec<_>>(),
        after
            .lines()
            .filter(|held| !held.contains("refs/heads/main"))
            .collect::<Vec<_>>(),
        "rejoin writes nothing to origin"
    );
}
