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

pub fn command(root: &Path, args: &[&str]) -> Output {
    let mut command = super::command::plumb(root, args);
    let binary = Path::new(env!("CARGO_BIN_EXE_plumb"))
        .parent()
        .expect("Plumb binary parent");
    let path = format!(
        "{}:{}",
        binary.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    command
        .env("HARNESS_RUN_TIMEOUT_MS", "1000")
        .env("PATH", path)
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
