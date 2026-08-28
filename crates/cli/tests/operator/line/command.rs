use std::path::Path;
use std::process::Command;

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
