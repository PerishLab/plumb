use std::path::Path;
use std::process::Command;

#[path = "../../../../src/command/ship/transport/sources.rs"]
mod sources;

#[test]
fn resources() {
    let seat = tempfile::tempdir().expect("repository");
    let root = seat.path();
    git(root, &["init", "-q"]);
    for (path, body) in [
        ("Cargo.toml", "[workspace]\nmembers = [\"crates/cli\"]\n"),
        (
            "crates/cli/Cargo.toml",
            "[package]\nname = \"probe\"\nversion = \"1.0.0\"\n",
        ),
        ("crates/cli/src/main.rs", "fn main() {}"),
        ("crates/cli/cookbook/index.txt", "first"),
        ("crates/cli/help/usage.txt", "help"),
        ("crates/cli/assets/icon.txt", "icon"),
        ("crates/cli/tests/probe.rs", "test"),
        ("AGENTS.md", "operations"),
    ] {
        write(root, path, body);
    }
    git(root, &["add", "."]);
    git(
        root,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.test",
            "commit",
            "-qm",
            "source",
        ],
    );
    let paths = sources::read(root).expect("production inputs");
    assert!(paths.contains("crates/cli/cookbook"));
    assert!(paths.contains("crates/cli/help"));
    assert!(paths.contains("crates/cli/assets"));
    let first = digest(root);
    write(root, "crates/cli/tests/probe.rs", "changed test");
    write(root, "AGENTS.md", "changed operations");
    git(root, &["add", "."]);
    assert_eq!(digest(root), first);
    write(root, "crates/cli/cookbook/index.txt", "second");
    git(root, &["add", "."]);
    assert_ne!(digest(root), first);
}

fn digest(root: &Path) -> String {
    let paths = sources::read(root).expect("production inputs");
    let depot = crate::support::depot(&[]);
    let mut command = Command::new(env!("CARGO_BIN_EXE_plumb"));
    command
        .env("PLUMB_HOME", depot.path())
        .args(["workflow", "plan"])
        .arg(root);
    for path in paths {
        command.arg("--root").arg(format!("ship/binary={path}"));
    }
    let output = command.output().expect("input plan");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let plan: serde_json::Value = serde_json::from_slice(&output.stdout).expect("plan JSON");
    plan["actions"]
        .as_array()
        .expect("actions")
        .iter()
        .find(|action| action["name"] == "ship/binary")
        .expect("binary action")["keys"]["workload"]
        .as_str()
        .expect("workload key")
        .to_string()
}

fn write(root: &Path, path: &str, body: &str) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    std::fs::write(path, body).expect("file");
}

fn git(root: &Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .status()
            .expect("git")
            .success()
    );
}
