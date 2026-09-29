use super::{Repo, cache, support};
use serde_json::Value;
use std::process::{Command, Output};

struct Runtime {
    root: tempfile::TempDir,
    home: tempfile::TempDir,
}

impl Runtime {
    fn new() -> Self {
        let root = cache::fixture();
        Repo::git(root.path(), &["branch", "-m", "main"]);
        Repo::git(
            root.path(),
            &[
                "remote",
                "set-url",
                "origin",
                "https://git.example.invalid/Example/runtime.git",
            ],
        );
        Repo::git(root.path(), &["commit", "-q", "-m", "fixture"]);
        Self {
            root,
            home: support::home(),
        }
    }

    fn command(&self) -> Command {
        let mut command = support::plumb();
        command
            .args(["guard", ".", "--json"])
            .current_dir(self.root.path())
            .env("PLUMB_HOME", self.home.path())
            .env("PLUMB_GUARD_STRENGTH", "full")
            .env("PLUMB_GUARD_BOUNDARY", "head");
        command
    }

    fn run(&self) -> Output {
        self.command().output().expect("runtime guard")
    }
}

fn report(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "runtime report: {error}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

#[test]
fn evidence() {
    let fixture = Runtime::new();
    let first = fixture.run();
    assert!(
        first.status.success(),
        "{}{}",
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );
    let first = report(&first);
    assert_eq!(first["schema"], "plumb.guard-runtime/v1");
    assert_eq!(first["ok"], true);
    assert_eq!(first["strength"], "full");
    assert_eq!(first["boundary"], "head");
    assert_eq!(first["guard"]["repository"], "Example/runtime");
    assert_eq!(first["guard"]["tree"].as_str().unwrap().len(), 40);
    assert_eq!(first["commit"].as_str().unwrap().len(), 40);
    assert_eq!(first["digest"].as_str().unwrap().len(), 64);
    assert_ne!(first["digest"], first["guard"]["digest"]);

    let second = fixture.run();
    assert!(second.status.success());
    assert_eq!(
        first,
        report(&second),
        "exact HEAD evidence is deterministic"
    );
}

#[test]
fn refusals() {
    let fixture = Runtime::new();

    let partial = fixture
        .command()
        .env_remove("PLUMB_GUARD_BOUNDARY")
        .output()
        .expect("partial");
    assert!(!partial.status.success());
    assert!(
        report(&partial)["message"]
            .as_str()
            .unwrap()
            .contains("must be declared together")
    );

    let unknown = fixture
        .command()
        .env("PLUMB_GUARD_STRENGTH", "quick")
        .output()
        .expect("unknown");
    assert!(!unknown.status.success());
    assert!(
        report(&unknown)["message"]
            .as_str()
            .unwrap()
            .contains("must be full")
    );

    let explicit = fixture
        .command()
        .args(["--base", "HEAD", "--head", "HEAD", "--write", "."])
        .output()
        .expect("explicit");
    assert!(!explicit.status.success());
    assert!(
        report(&explicit)["message"]
            .as_str()
            .unwrap()
            .contains("disagree")
    );

    std::fs::write(fixture.root.path().join("dirty"), "dirty\n").expect("dirty");
    let dirty = fixture.run();
    assert!(!dirty.status.success());
    assert!(
        report(&dirty)["message"]
            .as_str()
            .unwrap()
            .contains("exact clean HEAD")
    );
}

#[test]
fn neutral() {
    let fixture = Runtime::new();
    let output = fixture
        .command()
        .env_remove("PLUMB_GUARD_STRENGTH")
        .env_remove("PLUMB_GUARD_BOUNDARY")
        .env("CI", "true")
        .env("GITHUB_ACTIONS", "true")
        .output()
        .expect("local guard");
    assert!(!output.status.success(), "main still has the local refusal");
    let finding = report(&output);
    assert_eq!(finding["schema"], "plumb.guard-finding/v1");
    assert_eq!(finding["code"], "guard.integration-branch");
}
