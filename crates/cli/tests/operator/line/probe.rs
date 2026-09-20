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
