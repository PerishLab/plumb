use super::forgejo::{Court, serve};
use std::path::Path;
use std::process::{Command, Output};

fn repo(root: &Path, origin: &str) {
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

fn command(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(args)
        .current_dir(root)
        .env("FORGEJO_TOKEN", "test-token")
        .env("HARNESS_RUN_POLL_MS", "1")
        .env("HARNESS_RUN_TIMEOUT_MS", "100")
        .output()
        .expect("plumb")
}

fn run(command: &mut Command) {
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
    let (url, calls) = serve(Court::Dispatch, 2);
    repo(fixture.path(), &format!("{url}/test/probe.git"));
    let output = command(
        fixture.path(),
        &[
            "ship",
            "binary",
            "dispatch",
            "--version",
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
    assert!(
        text.contains("triggered release-exact.yml run 88"),
        "{text}"
    );
    assert!(text.contains("run 88: success"), "{text}");
    let calls = calls.lock().expect("calls");
    assert!(calls.iter().any(|call| call.contains("/dispatches ")));
    assert!(calls.iter().any(|call| call.contains("/actions/runs/88 ")));
}

#[test]
fn nested() {
    let fixture = tempfile::tempdir().expect("fixture");
    let (url, calls) = serve(Court::Nested(true), 3);
    repo(fixture.path(), &format!("{url}/test/probe.git"));
    let output = command(
        fixture.path(),
        &[
            "ship",
            "binary",
            "dispatch",
            "--version",
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
            .any(|call| call.contains("/actions/tasks?"))
    );
}

#[test]
fn failure() {
    let fixture = tempfile::tempdir().expect("fixture");
    let (url, _) = serve(Court::Failed, 3);
    repo(fixture.path(), &format!("{url}/test/probe.git"));
    let output = command(
        fixture.path(),
        &[
            "ship",
            "binary",
            "dispatch",
            "--version",
            "v1.2.0-nightly.3",
            "--watch",
        ],
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("failed tasks: build"));
}

#[test]
fn blocked() {
    let fixture = tempfile::tempdir().expect("fixture");
    let (url, _) = serve(Court::Nested(false), 3);
    repo(fixture.path(), &format!("{url}/test/probe.git"));
    let output = command(
        fixture.path(),
        &[
            "ship",
            "binary",
            "dispatch",
            "--version",
            "v1.2.0-nightly.5",
            "--watch",
        ],
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("failed tasks: build"));
}

#[test]
fn pagination() {
    let fixture = tempfile::tempdir().expect("fixture");
    let (url, calls) = serve(Court::Paged, 4);
    repo(fixture.path(), &format!("{url}/test/probe.git"));
    let output = command(
        fixture.path(),
        &[
            "ship",
            "binary",
            "dispatch",
            "--version",
            "v1.2.0-nightly.4",
            "--watch",
        ],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        calls
            .lock()
            .expect("calls")
            .iter()
            .any(|call| call.contains("page=2"))
    );
}

#[test]
fn protection() {
    let fixture = tempfile::tempdir().expect("fixture");
    let (url, _) = serve(Court::Prepare(true), 5);
    repo(fixture.path(), &format!("{url}/test/probe.git"));
    let output = command(fixture.path(), &["stable", "prepare", "--version", "1.2.0"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("prepared release/v1.2.0 from main"));
}

#[test]
fn drift() {
    let fixture = tempfile::tempdir().expect("fixture");
    let (url, _) = serve(Court::Prepare(false), 3);
    repo(fixture.path(), &format!("{url}/test/probe.git"));
    let output = command(fixture.path(), &["stable", "prepare", "--version", "1.2.0"]);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("branch protection did not read back byte-for-byte")
    );
}

#[test]
fn freedom() {
    let fixture = tempfile::tempdir().expect("fixture");
    repo(
        fixture.path(),
        "ssh://git@git.perish.top/PerishLab/probe.git",
    );
    let output = command(
        fixture.path(),
        &[
            "ship",
            "binary",
            "dispatch",
            "--version",
            "v1.2.0-nightly.9",
            "--dry-run",
        ],
    );
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("ref=refs/tags/v1.2.0-nightly.9"), "{text}");
    assert!(text.contains("inputs={}"), "{text}");
}

#[test]
fn strict() {
    let fixture = tempfile::tempdir().expect("fixture");
    repo(
        fixture.path(),
        "ssh://git@git.perish.top/PerishLab/probe.git",
    );
    let output = command(
        fixture.path(),
        &[
            "ship",
            "binary",
            "dispatch",
            "--version",
            "v1.2.0",
            "--promotion-version",
            "v1.2.0-beta.4",
            "--dry-run",
        ],
    );
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("ref=release/v1.2.0"), "{text}");
    assert!(!text.contains(r#""version""#), "{text}");
}
