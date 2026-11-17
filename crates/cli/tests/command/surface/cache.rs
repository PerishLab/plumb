#![cfg(target_os = "linux")]

use super::super::configuration::support;
use serde_json::Value;
use std::path::Path;
use std::process::Command;

fn command(root: &Path, name: &str) -> Command {
    let mut command = support::plumb();
    command
        .args(["cache", "run", "--json", "--output"])
        .arg(root.join(name))
        .args(["--cwd", root.to_str().unwrap()])
        .env("PLUMB_HOME", root.join("home"))
        .env_remove("CARGO_TARGET_DIR");
    command
}

fn report(root: &Path, name: &str) -> Value {
    serde_json::from_slice(&std::fs::read(root.join(name).join("result.json")).unwrap()).unwrap()
}

fn cleaned(root: &Path, name: &str) {
    let output = root.join(name);
    let invocation: Value =
        serde_json::from_slice(&std::fs::read(output.join("invocation.json")).unwrap()).unwrap();
    assert!(!Path::new(invocation["directory"].as_str().unwrap()).exists());
    assert!(!Path::new(invocation["registry"].as_str().unwrap()).exists());
    assert_eq!(report(root, name)["cleanup"], true);
}

#[test]
fn lifecycle() {
    let seat = tempfile::tempdir().unwrap();
    let root = seat.path();
    std::fs::create_dir(root.join("target")).unwrap();
    std::fs::write(root.join("target/foreign"), "preserve").unwrap();
    for (name, script, expected) in [
        (
            "success",
            "mkdir -p \"$CARGO_TARGET_DIR\"; echo payload >\"$CARGO_TARGET_DIR/data\"; printf output; printf diagnostic >&2",
            0,
        ),
        ("failure", "exit 23", 23),
        ("signal", "kill -KILL $$", 137),
        ("timeout", "sleep 30", 124),
    ] {
        let output = command(root, name)
            .args(["--timeout", "1", "--", "/bin/sh", "-c", script])
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(expected),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(report(root, name)["exit"], expected);
        cleaned(root, name);
    }
    assert_eq!(
        std::fs::read(root.join("success/stdout.log")).unwrap(),
        b"output"
    );
    assert_eq!(
        std::fs::read(root.join("success/stderr.log")).unwrap(),
        b"diagnostic"
    );
    assert_eq!(
        std::fs::read(root.join("target/foreign")).unwrap(),
        b"preserve"
    );
}

#[test]
fn background() {
    let seat = tempfile::tempdir().unwrap();
    let root = seat.path();
    let output = command(root, "background")
        .args(["--", "/bin/sh", "-c", "sleep 30 & echo $!; exit 0"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let pid = std::fs::read_to_string(root.join("background/stdout.log")).unwrap();
    let path = Path::new("/proc").join(pid.trim()).join("stat");
    if let Ok(stat) = std::fs::read_to_string(path) {
        assert!(stat.rsplit_once(") ").unwrap().1.starts_with(['Z', 'X']));
    }
    cleaned(root, "background");
}

#[test]
fn ownership() {
    let seat = tempfile::tempdir().unwrap();
    let root = seat.path();
    let output = command(root, "changed")
        .args([
            "--",
            "/bin/sh",
            "-c",
            "printf foreign >\"$PLUMB_HOME/../plumb.json\"",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(125));
    let invocation: Value =
        serde_json::from_slice(&std::fs::read(root.join("changed/invocation.json")).unwrap())
            .unwrap();
    assert!(Path::new(invocation["directory"].as_str().unwrap()).exists());
    assert!(Path::new(invocation["registry"].as_str().unwrap()).exists());
    assert_eq!(report(root, "changed")["process"], true);
    assert_eq!(report(root, "changed")["cleanup"], false);
}

#[test]
fn refusal() {
    let seat = tempfile::tempdir().unwrap();
    let root = seat.path();
    std::fs::create_dir(root.join("existing")).unwrap();
    std::fs::write(root.join("existing/foreign"), "preserve").unwrap();
    for (name, bound) in [("existing", false), ("bound", true)] {
        let mut command = command(root, name);
        if bound {
            command.env("CARGO_TARGET_DIR", root.join("foreign"));
        }
        let output = command.args(["--", "/bin/true"]).output().unwrap();
        assert_eq!(output.status.code(), Some(125));
    }
    assert!(!root.join("bound").exists());
    assert_eq!(
        std::fs::read(root.join("existing/foreign")).unwrap(),
        b"preserve"
    );
}

#[test]
fn consumer() {
    let seat = tempfile::tempdir().unwrap();
    let root = seat.path();
    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='consumer'\nversion='0.0.0'\nedition='2024'\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/lib.rs"),
        "#[test] fn proof() { assert_eq!(2 + 2, 4); }\n",
    )
    .unwrap();
    let output = command(root, "cargo")
        .args(["--", "cargo", "test", "--offline"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&std::fs::read(root.join("cargo/stderr.log")).unwrap())
    );
    cleaned(root, "cargo");
    assert!(!root.join("target").exists());
}

#[test]
fn arguments() {
    let seat = tempfile::tempdir().unwrap();
    let root = seat.path();
    let values = ["$HOME", "space value", "%n", "$(touch escaped)"];
    let output = command(root, "literal")
        .args(["--", "/bin/sh", "-c", "printf '%s\\n' \"$@\"", "--"])
        .args(values)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        std::fs::read_to_string(root.join("literal/stdout.log")).unwrap(),
        values.join("\n") + "\n"
    );
    assert!(!root.join("escaped").exists());
    cleaned(root, "literal");
}

#[test]
fn cancellation() {
    use std::process::Stdio;
    use std::time::{Duration, Instant};
    let seat = tempfile::tempdir().unwrap();
    let root = seat.path();
    let mut left = command(root, "left")
        .args([
            "--timeout",
            "5",
            "--",
            "/bin/sh",
            "-c",
            "echo ready; sleep 30",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut right = command(root, "right")
        .args(["--timeout", "5", "--", "/bin/sh", "-c", "sleep 1; exit 0"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let until = Instant::now() + Duration::from_secs(3);
    while !std::fs::read_to_string(root.join("left/stdout.log"))
        .is_ok_and(|text| text.contains("ready"))
    {
        assert!(Instant::now() < until);
        assert!(left.try_wait().unwrap().is_none());
        std::thread::sleep(Duration::from_millis(10));
    }
    let sent = support::outside("kill")
        .args(["-TERM", &left.id().to_string()])
        .status()
        .unwrap();
    assert!(sent.success());
    assert_eq!(left.wait().unwrap().code(), Some(143));
    assert!(right.wait().unwrap().success());
    cleaned(root, "left");
    cleaned(root, "right");
}
