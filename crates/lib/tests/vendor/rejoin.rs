use plumb::land::rejoin::report;
use std::path::{Path, PathBuf};
use std::process::Command;

struct Line {
    #[allow(dead_code)]
    fixture: tempfile::TempDir,
    work: PathBuf,
    origin: PathBuf,
}

impl Line {
    fn new() -> Self {
        let fixture = tempfile::tempdir().expect("fixture");
        let origin = fixture.path().join("fixture.git");
        let work = fixture.path().join("work");
        std::fs::create_dir_all(&work).expect("work");
        git(fixture.path(), &["init", "-q", "--bare", "fixture.git"]);
        git(&work, &["init", "-q", "-b", "main"]);
        git(&work, &["config", "user.name", "Plumb Test"]);
        git(&work, &["config", "user.email", "plumb@example.invalid"]);
        git(
            &work,
            &[
                "remote",
                "add",
                "origin",
                &origin.to_str().expect("utf8").replace('\\', "/"),
            ],
        );
        let held = Self {
            fixture,
            work,
            origin,
        };
        held.commit("README.md", "base\n", "base");
        held
    }

    fn commit(&self, path: &str, text: &str, message: &str) -> String {
        std::fs::write(self.work.join(path), text).expect("write");
        git(&self.work, &["add", "-A"]);
        git(&self.work, &["commit", "-q", "-m", message]);
        head(&self.work, "HEAD")
    }

    fn stable(&self, branch: &str, marker: &str) {
        git(&self.work, &["tag", "-a", marker, branch, "-m", marker]);
        let mut pushed = vec!["push", "-q", "origin", "main", marker];
        if branch != "main" {
            pushed.push(branch);
        }
        git(&self.work, &pushed);
    }

    fn remote(&self) -> String {
        git(
            &self.origin,
            &["for-each-ref", "--format=%(refname) %(objectname)"],
        )
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

fn head(root: &Path, reference: &str) -> String {
    git(root, &["rev-parse", reference])
}

fn cut(line: &Line, branch: &str, text: &str) -> String {
    git(&line.work, &["checkout", "-q", "-b", branch, "main"]);
    let commit = line.commit("fix.txt", text, "fix on the release line");
    git(&line.work, &["checkout", "-q", "main"]);
    commit
}

#[test]
fn unmarked() {
    let line = Line::new();
    git(&line.work, &["push", "-q", "origin", "main"]);
    let report = report(&line.work).expect("report");
    assert_eq!(report.state, "unmarked");
}

#[test]
fn home() {
    let line = Line::new();
    line.stable("main", "v1.0.0");
    let report = report(&line.work).expect("report");
    assert_eq!(
        (report.state, report.marker.as_deref()),
        ("home", Some("v1.0.0"))
    );
}

#[test]
fn owed() {
    let line = Line::new();
    let released = cut(&line, "release/v1.0.0", "fixed\n");
    line.stable("release/v1.0.0", "v1.0.0");
    let before = line.remote();
    let owed = report(&line.work).expect("report");
    assert_eq!(
        (owed.state, owed.commit.as_deref(), owed.conflicted),
        ("owed", Some(released.as_str()), false)
    );
    assert_eq!(owed.lacking, ["fix.txt"]);
    assert_eq!(line.remote(), before, "rejoin pushes nothing");
}

#[test]
fn squashed() {
    let line = Line::new();
    let released = cut(&line, "release/v1.0.0", "fixed\n");
    line.stable("release/v1.0.0", "v1.0.0");
    let main = line.commit("fix.txt", "fixed\n", "Squash the release fix into main");
    git(&line.work, &["push", "-q", "origin", "main"]);
    let before = line.remote();
    let home = report(&line.work).expect("report");
    assert_eq!(
        (home.state, home.commit.as_deref(), home.main.as_deref()),
        ("home", Some(released.as_str()), Some(main.as_str()))
    );
    assert!(home.lacking.is_empty(), "{:?}", home.lacking);
    assert_eq!(line.remote(), before, "rejoin pushes nothing");
    let listed = git(&line.origin, &["branch", "--list", "rejoin/*"]);
    assert!(listed.is_empty(), "no rejoin branch: {listed}");
}

#[test]
fn conflicting() {
    let line = Line::new();
    cut(&line, "release/v1.0.0", "fixed on the line\n");
    line.stable("release/v1.0.0", "v1.0.0");
    line.commit("fix.txt", "fixed otherwise\n", "Fix main otherwise");
    git(&line.work, &["push", "-q", "origin", "main"]);
    let owed = report(&line.work).expect("report");
    assert_eq!((owed.state, owed.conflicted), ("owed", true));
    assert_eq!(owed.lacking, ["fix.txt"]);
}
