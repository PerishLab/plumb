mod expanding;
mod flight;

use super::world::{Court, serve};
use std::path::Path;
use std::process::{Command, Output};

pub fn repo(root: &Path, origin: &str) {
    std::fs::write(
        root.join("plumb.toml"),
        r#"[release]
product = "probe"
authority = "https://releases.test"
binaries = ["probe"]
targets = ["x86_64-unknown-linux-gnu"]
"#,
    )
    .expect("manifest");
    run(Command::new("git").args(["init", "-q"]).current_dir(root));
    run(Command::new("git")
        .args(["remote", "add", "origin", origin])
        .current_dir(root));
}

pub(super) fn marked(root: &Path, bare: &Path, forge: &str, version: &str) {
    run(Command::new("git").args(["init", "-q", "--bare"]).arg(bare));
    repo(root, &format!("file://{}", bare.display()));
    run(Command::new("git")
        .args(["config", "user.name", "Plumb Test"])
        .current_dir(root));
    run(Command::new("git")
        .args(["config", "user.email", "plumb@example.test"])
        .current_dir(root));
    run(Command::new("git")
        .args(["config", "plumb.test-forgejo-url", forge])
        .current_dir(root));
    std::fs::create_dir_all(root.join(".plumb/releases/v1.2.0")).expect("datum root");
    std::fs::write(
        root.join(".plumb/releases/v1.2.0/datum.toml"),
        "schema = 1\nversion = \"v1.2.0\"\n",
    )
    .expect("datum");
    run(Command::new("git").args(["add", "."]).current_dir(root));
    run(Command::new("git")
        .args(["commit", "-qm", "candidate"])
        .current_dir(root));
    run(Command::new("git")
        .args(["tag", "-a", version, "-m", &format!("probe {version}")])
        .current_dir(root));
    run(Command::new("git")
        .args(["push", "-q", "origin", "HEAD:refs/heads/main"])
        .current_dir(root));
    run(Command::new("git")
        .args(["push", "-q", "origin", "HEAD:refs/heads/release/v1.2.0"])
        .current_dir(root));
    run(Command::new("git")
        .args(["push", "-q", "origin", &format!("refs/tags/{version}")])
        .current_dir(root));
}

pub fn command(root: &Path, args: &[&str]) -> Output {
    let mut command = super::command::plumb(root, args);
    command
        .env("HARNESS_RUN_TIMEOUT_MS", "1000")
        .output()
        .expect("plumb")
}

pub fn run(command: &mut Command) {
    let output = command.output().expect("command");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn dispatch() {
    let fixture = tempfile::tempdir().expect("fixture");
    let bare = tempfile::tempdir().expect("bare");
    let (url, calls) = serve(Court::Dispatch, 7);
    marked(fixture.path(), bare.path(), &url, "v1.2.0-nightly.1");
    let output = command(
        fixture.path(),
        &[
            "ship",
            "dispatch",
            "--marker",
            "v1.2.0-nightly.1",
            "--watch",
        ],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("triggered ship.yml run 88"), "{text}");
    assert!(text.contains("run 88: success"), "{text}");
    let calls = calls.lock().expect("calls");
    assert!(calls.iter().any(|call| call.contains("/dispatches ")));
    assert!(
        calls
            .iter()
            .any(|call| call.contains("/actions/runs/7/jobs/0/attempt/1 "))
    );
}

#[test]
fn nested() {
    let fixture = tempfile::tempdir().expect("fixture");
    let bare = tempfile::tempdir().expect("bare");
    let (url, calls) = serve(Court::Nested(true), 7);
    marked(fixture.path(), bare.path(), &url, "v1.2.0-nightly.2");
    let output = command(
        fixture.path(),
        &[
            "ship",
            "dispatch",
            "--marker",
            "v1.2.0-nightly.2",
            "--watch",
        ],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("run 88: success"));
    assert!(
        calls
            .lock()
            .expect("calls")
            .iter()
            .any(|call| call.contains("/actions/runs/7/jobs/0/attempt/1 "))
    );
}

#[test]
fn failure() {
    let fixture = tempfile::tempdir().expect("fixture");
    let bare = tempfile::tempdir().expect("bare");
    let (url, _) = serve(Court::Failed, 3);
    marked(fixture.path(), bare.path(), &url, "v1.2.0-nightly.3");
    let output = command(
        fixture.path(),
        &[
            "ship",
            "dispatch",
            "--marker",
            "v1.2.0-nightly.3",
            "--watch",
        ],
    );
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("triggered ship.yml run 88"),
        "dispatch identity should be visible before a watched failure"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("failed tasks: build"));
}

#[test]
fn blocked() {
    let fixture = tempfile::tempdir().expect("fixture");
    let bare = tempfile::tempdir().expect("bare");
    let (url, _) = serve(Court::Nested(false), 3);
    marked(fixture.path(), bare.path(), &url, "v1.2.0-nightly.5");
    let output = command(
        fixture.path(),
        &[
            "ship",
            "dispatch",
            "--marker",
            "v1.2.0-nightly.5",
            "--watch",
        ],
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("failed tasks: build"));
}

#[test]
fn drift() {
    let fixture = tempfile::tempdir().expect("fixture");
    let bare = tempfile::tempdir().expect("bare");
    let cut = fixture.path().join("cut");
    let (url, _) = serve(Court::Prepare(false, cut.clone()), 5);
    let origin = format!("{url}/test/probe.git");
    let head = super::datum::lined(fixture.path(), &origin, bare.path(), "release/v1.2.0");
    std::fs::write(&cut, &head).expect("cut");
    let output = command(
        fixture.path(),
        &["version", "prepare", "--version", "1.2.0"],
    );
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("branch protection did not read back byte-for-byte")
    );
}

#[test]
fn freedom() {
    let fixture = tempfile::tempdir().expect("fixture");
    let bare = tempfile::tempdir().expect("bare");
    marked(
        fixture.path(),
        bare.path(),
        "https://git.perish.top",
        "v1.2.0-nightly.9",
    );
    let output = command(
        fixture.path(),
        &[
            "ship",
            "dispatch",
            "--marker",
            "v1.2.0-nightly.9",
            "--dry-run",
        ],
    );
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("ship.yml"), "{text}");
    assert!(text.contains(r#""marker":"v1.2.0-nightly.9""#), "{text}");
}

#[test]
fn strict() {
    let fixture = tempfile::tempdir().expect("fixture");
    repo(fixture.path(), "ssh://git@127.0.0.1:9/PerishLab/probe.git");
    let output = command(
        fixture.path(),
        &[
            "ship",
            "binary",
            "dispatch",
            "--version",
            "v1.2.0",
            "--dry-run",
        ],
    );
    let text = String::from_utf8_lossy(&output.stdout);
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "a preview that cannot read the line must refuse: {text}"
    );
    assert!(
        !text.contains("branch_protections"),
        "a refused preview must not have printed a plan: {text}"
    );
    assert!(!said.is_empty(), "a refusal must say why");
}

#[test]
fn stamped() {
    let fixture = tempfile::tempdir().expect("fixture");
    repo(fixture.path(), "ssh://git@127.0.0.1:9/PerishLab/probe.git");

    let exact = command(
        fixture.path(),
        &[
            "release",
            "stamp",
            "--version",
            "v1.2.0-beta.1",
            "--dry-run",
        ],
    );
    let printed = String::from_utf8_lossy(&exact.stdout).to_string();
    assert!(
        !exact.status.success(),
        "a preview that cannot read the line must refuse: {printed}"
    );
    assert!(
        !printed.contains("git tag"),
        "a refused preview must not have printed a plan: {printed}"
    );
}
