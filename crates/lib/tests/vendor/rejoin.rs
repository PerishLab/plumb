use plumb::guard::{Action, Descriptor, TRAILER};
use plumb::land::rejoin::{plan, run};
use std::path::{Path, PathBuf};
use std::process::Command;

const CARRIED: &[(&str, &str)] = &[("rules/fixture.toml", "")];
const DIGEST: &str = "0000000000000000000000000000000000000000000000000000000000000000";

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
            &["remote", "add", "origin", origin.to_str().expect("utf8")],
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

    fn guarded(&self, path: &str, text: &str) -> String {
        std::fs::write(self.work.join(path), text).expect("write");
        git(&self.work, &["add", "-A"]);
        let tree = git(&self.work, &["write-tree"]);
        plumb::depot::carry(CARRIED);
        let action = Action {
            name: "guard/test".into(),
            input: DIGEST.into(),
            world: DIGEST.into(),
        };
        let proof = Descriptor::new(&self.work, tree, vec![action]).expect("proof");
        let message = format!("guarded\n\n{TRAILER} {}", proof.encode().expect("token"));
        git(&self.work, &["commit", "-q", "-m", &message]);
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

    fn remote(&self, reference: &str) -> String {
        head(&self.origin, reference)
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
    let report = plan(&line.work).expect("plan");
    assert_eq!(report.state, "unmarked");
}

#[test]
fn home() {
    let line = Line::new();
    line.stable("main", "v1.0.0");
    let report = plan(&line.work).expect("plan");
    assert_eq!(
        (report.state, report.marker.as_deref()),
        ("home", Some("v1.0.0"))
    );
}

#[test]
fn diverged() {
    let line = Line::new();
    cut(&line, "release/v1.0.0", "only on the line\n");
    line.stable("release/v1.0.0", "v1.0.0");
    let refusal = plan(&line.work).expect_err("a line main lacks refuses");
    assert_eq!(refusal.kind, "diverged", "{}", refusal.message);
}

#[test]
fn topology() {
    let line = Line::new();
    let released = cut(&line, "release/v1.0.0", "fixed\n");
    line.stable("release/v1.0.0", "v1.0.0");
    let main = line.guarded("fix.txt", "fixed\n");
    git(&line.work, &["push", "-q", "origin", "main"]);
    let report = plan(&line.work).expect("plan");
    assert_eq!(report.state, "owed");
    let refusal = run(&line.work).expect_err("a local origin is not GitHub");
    assert_eq!(refusal.kind, "remote", "{}", refusal.message);
    settled(&line, &main, &released);
}

#[test]
fn landed() {
    let line = Line::new();
    let released = cut(&line, "release/v1.0.0", "fixed\n");
    line.stable("release/v1.0.0", "v1.0.0");
    git(&line.work, &["checkout", "-q", "-b", "land/fix", "main"]);
    line.guarded("fix.txt", "fixed\n");
    git(&line.work, &["checkout", "-q", "main"]);
    git(
        &line.work,
        &["merge", "-q", "--no-ff", "land/fix", "-m", "merge"],
    );
    let main = head(&line.work, "HEAD");
    git(&line.work, &["push", "-q", "origin", "main"]);
    run(&line.work).expect_err("a local origin is not GitHub");
    settled(&line, &main, &released);
}

fn settled(line: &Line, main: &str, released: &str) {
    let rejoin = line.remote("refs/heads/rejoin/v1.0.0");
    assert_eq!(
        git(&line.origin, &["rev-list", "--parents", "-n", "1", &rejoin]),
        format!("{rejoin} {main} {released}")
    );
    assert_eq!(
        line.remote(&format!("{rejoin}^{{tree}}")),
        line.remote(&format!("{main}^{{tree}}"))
    );
    let message = git(&line.origin, &["show", "-s", "--format=%B", &rejoin]);
    assert!(message.starts_with("Rejoin v1.0.0"), "{message}");
    assert!(
        message.contains(&format!("Rejoin-Source: v1.0.0@{released}")),
        "{message}"
    );
    assert_eq!(message.matches(TRAILER).count(), 1, "{message}");
}

#[test]
fn unguarded() {
    let line = Line::new();
    cut(&line, "release/v1.0.0", "fixed\n");
    line.stable("release/v1.0.0", "v1.0.0");
    line.commit("fix.txt", "fixed\n", "fix without a proof");
    git(&line.work, &["push", "-q", "origin", "main"]);
    let refusal = run(&line.work).expect_err("main without a proof refuses");
    assert_eq!(refusal.kind, "guard", "{}", refusal.message);
    let listed = git(&line.origin, &["branch", "--list", "rejoin/*"]);
    assert!(listed.is_empty(), "nothing is pushed: {listed}");
}
