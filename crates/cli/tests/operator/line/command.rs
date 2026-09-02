use std::path::Path;
use std::process::Command;

#[path = "../../support.rs"]
mod support;

fn forge(root: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["config", "--get", "plumb.test-forgejo-url"])
        .current_dir(root)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn plumb(root: &Path, args: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_plumb"));
    command
        .args(args)
        .current_dir(root)
        .env_remove("FORGEJO_URL");
    for (name, value) in [
        ("FORGEJO_TOKEN", "test-token"),
        ("HARNESS_RUN_POLL_MS", "1"),
        ("HARNESS_RUN_TIMEOUT_MS", "100"),
    ] {
        command.env(name, value);
    }
    if let Some(forge) = forge(root) {
        command.env("FORGEJO_URL", forge);
    }
    command
}

#[test]
fn scoped() {
    let fixture = tempfile::tempdir().expect("fixture");
    let bare = tempfile::tempdir().expect("bare");
    let (url, calls) = super::world::serve(super::world::Court::Dispatch, 10);
    super::stable::marked(fixture.path(), bare.path(), &url, "v1.2.0-nightly.4");
    let output = plumb(
        fixture.path(),
        &[
            "ship",
            "dispatch",
            "--marker",
            "v1.2.0-nightly.4",
            "--watch",
        ],
    )
    .env("HARNESS_RUN_TIMEOUT_MS", "1000")
    .output()
    .expect("plumb");
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
            .any(|call| call.contains("/actions/runs/7/jobs/0/attempt/1 "))
    );
    assert!(
        calls
            .lock()
            .expect("calls")
            .iter()
            .all(|call| !call.contains("/actions/tasks?"))
    );
}

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("git should run");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn recovery() {
    let held = tempfile::tempdir().expect("fixture");
    let home = support::depot(&[]);
    let remote = held.path().join("remote.git");
    let seat = held.path().join("seat");
    git(
        held.path(),
        &["init", "--bare", remote.to_str().expect("remote")],
    );
    std::fs::create_dir(&seat).expect("seat");
    git(&seat, &["init", "-q"]);
    git(&seat, &["config", "user.name", "Plumb"]);
    git(&seat, &["config", "user.email", "plumb@example.invalid"]);
    git(
        &seat,
        &["remote", "add", "origin", remote.to_str().expect("remote")],
    );
    std::fs::write(seat.join("member"), "base\n").expect("base member");
    git(&seat, &["add", "member"]);
    git(&seat, &["commit", "-q", "-m", "base"]);
    git(&seat, &["branch", "-M", "release/v1.0.0"]);
    git(&seat, &["push", "-q", "-u", "origin", "release/v1.0.0"]);
    let base = git(&seat, &["rev-parse", "HEAD"]);

    git(&seat, &["checkout", "-q", "-b", "candidate-one"]);
    std::fs::write(seat.join("member"), "one\n").expect("first candidate");
    git(&seat, &["commit", "-qam", "one"]);
    let one = git(&seat, &["rev-parse", "HEAD"]);
    git(&seat, &["checkout", "-q", "release/v1.0.0"]);
    git(&seat, &["checkout", "-q", "-b", "candidate-two"]);
    std::fs::write(seat.join("member"), "two\n").expect("second candidate");
    git(&seat, &["commit", "-qam", "two"]);
    let two = git(&seat, &["rev-parse", "HEAD"]);
    git(&seat, &["checkout", "-q", "release/v1.0.0"]);

    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args([
            "version",
            "pick",
            "--version",
            "1.0.0",
            "--commit",
            &one,
            "--commit",
            &two,
        ])
        .current_dir(&seat)
        .env("PLUMB_HOME", home.path())
        .output()
        .expect("plumb should run");
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("cherry-pick candidates failed"), "{error}");
    assert_eq!(git(&seat, &["rev-parse", "HEAD"]), base);
    assert!(git(&seat, &["status", "--short"]).is_empty());
    assert!(!seat.join(".git/CHERRY_PICK_HEAD").exists());
}
