use super::probe::{repo, run};
use std::path::PathBuf;
use std::process::{Command, Output};

const ORIGIN: &str = "git@github.com:PerishLab/probe.git";
const TRANSPORT: &str = "#!/bin/sh\nwhile [ \"$#\" -gt 1 ]; do shift; done\ncd \"$(dirname \"$0\")\" && eval \"git ${1#git-}\"\n";

struct Hub {
    fixture: tempfile::TempDir,
}

impl Hub {
    fn new() -> Self {
        let held = Self {
            fixture: tempfile::tempdir().expect("fixture"),
        };
        std::fs::write(held.transport(), TRANSPORT).expect("transport");
        std::fs::create_dir_all(held.work()).expect("work");
        run(Command::new("git")
            .args(["init", "-q", "--bare"])
            .arg(held.bare()));
        repo(&held.work(), ORIGIN);
        held.git(&["commit", "-q", "--allow-empty", "-m", "base"]);
        held.push("HEAD:refs/heads/main");
        held
    }

    fn transport(&self) -> PathBuf {
        self.fixture.path().join("transport.sh")
    }

    fn work(&self) -> PathBuf {
        self.fixture.path().join("work")
    }

    fn bare(&self) -> PathBuf {
        self.fixture.path().join("PerishLab/probe.git")
    }

    fn git(&self, args: &[&str]) {
        run(Command::new("git")
            .args([
                "-c",
                "user.name=Plumb",
                "-c",
                "user.email=plumb@example.invalid",
            ])
            .args(args)
            .current_dir(self.work()));
    }

    fn push(&self, reference: &str) {
        let bare = self.bare();
        self.git(&["push", "-q", bare.to_str().expect("utf8"), reference]);
    }

    fn mark(&self, marker: &str) {
        self.git(&["tag", "-a", marker, "-m", marker]);
        self.push(&format!("refs/tags/{marker}"));
    }

    fn line(&self, version: &str) {
        self.push(&format!("HEAD:refs/heads/release/{version}"));
    }

    fn plumb(&self, args: &[&str]) -> Output {
        super::command::plumb(&self.work(), args)
            .env(
                "GIT_SSH_COMMAND",
                format!("sh {}", self.transport().display()),
            )
            .env("GIT_SSH_VARIANT", "simple")
            .output()
            .expect("plumb")
    }

    fn refused(&self, args: &[&str], highest: &str) {
        let output = self.plumb(args);
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{args:?} must refuse");
        assert!(
            error.contains(&format!(
                "does not exceed {highest}, the highest marker origin holds"
            )),
            "{args:?}: {error}"
        );
        assert!(error.contains("[release.marker-ordered]"), "{error}");
        assert!(output.stdout.is_empty(), "a refusal prints no plan");
    }

    fn planned(&self, args: &[&str], step: &str) {
        let output = self.plumb(args);
        let plan = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(plan.contains(step), "{args:?}: {plan}");
    }

    fn below(highest: &str) {
        let hub = Self::new();
        hub.mark(highest);
        hub.refused(
            &["release", "open", "--version", "v1.0.1", "--dry-run"],
            highest,
        );
        hub.line("v1.0.1");
        hub.refused(
            &["release", "stamp", "--version", "v1.0.1-rc.1", "--dry-run"],
            highest,
        );
        hub.mark("v1.0.1-rc.1");
        hub.refused(
            &["ship", "dispatch", "--marker", "v1.0.1-rc.1", "--dry-run"],
            highest,
        );
    }
}

#[test]
fn stable() {
    Hub::below("v1.1.0");
}

#[test]
fn prerelease() {
    Hub::below("v1.1.0-rc.1");
}

#[test]
fn ascending() {
    let hub = Hub::new();
    hub.mark("v1.1.0-rc.1");
    hub.planned(
        &["release", "open", "--version", "v1.1.0", "--dry-run"],
        "refs/heads/release/v1.1.0",
    );
    hub.line("v1.1.0");
    hub.planned(
        &["release", "stamp", "--version", "v1.1.0-rc.2", "--dry-run"],
        "git tag -a v1.1.0-rc.2",
    );
    hub.planned(
        &["ship", "dispatch", "--marker", "v1.1.0-rc.1", "--dry-run"],
        "marker=v1.1.0-rc.1",
    );
}

#[test]
fn mainline() {
    let hub = Hub::new();
    let output = hub.plumb(&[
        "release",
        "open",
        "--version",
        "v1.0.1",
        "--from",
        "v1.0.0",
        "--dry-run",
    ]);
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "a fix line no longer opens");
    assert!(error.contains("unexpected argument '--from'"), "{error}");
}
