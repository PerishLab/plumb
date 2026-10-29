use sha2::{Digest, Sha256};
use std::path::Path;
use std::process::{Command, Output};

const DECLARATION: &str = r#"[windtun]
cases = ["writer"]
image = "ghcr.io/perishlab/images:stable"
[windtun.scenes]
authority = "https://depot.seatrial.perish.uk"
product = "seatrial"
channel = "stable"
"#;

fn inspect(root: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["windtun", "inspect", root.to_str().unwrap(), "--json"])
        .env("PLUMB_HOME", root)
        .env("PATH", root)
        .output()
        .expect("inspect should run without external tools")
}

#[test]
fn declaration() {
    let root = tempfile::tempdir().unwrap();
    let absent = inspect(root.path());
    assert!(absent.status.success(), "{:?}", absent);
    let result: serde_json::Value = serde_json::from_slice(&absent.stdout).unwrap();
    assert_eq!(result["schema"], plumb::windtun::SCHEMA);
    assert!(result["manifest"].is_null());
    assert!(result["windtun"].is_null());
    for text in [
        DECLARATION.to_string(),
        DECLARATION.replace("windtun", "guard.seatrial"),
    ] {
        std::fs::write(root.path().join("plumb.toml"), &text).unwrap();
        let output = inspect(root.path());
        assert!(output.status.success(), "{:?}", output);
        let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            result["manifest"],
            format!("{:x}", Sha256::digest(text.as_bytes()))
        );
        assert_eq!(result["windtun"]["cases"], serde_json::json!(["writer"]));
        assert_eq!(
            plumb::windtun::parse(&text),
            plumb::guard::seatrial::parse(&text)
        );
    }
}

#[test]
fn rejected() {
    let root = tempfile::tempdir().unwrap();
    for text in [
        DECLARATION.replace("[\"writer\"]", "[\"writer\", \"writer\"]"),
        DECLARATION.replace("[\"writer\"]", "[]"),
        DECLARATION.replace("channel = \"stable\"", "channel = \"v0.1.0\""),
        DECLARATION.replace("image =", "command ="),
        DECLARATION.to_owned() + &DECLARATION.replace("windtun", "guard.seatrial"),
        "[windtun".to_string(),
    ] {
        std::fs::write(root.path().join("plumb.toml"), text).unwrap();
        let output = inspect(root.path());
        assert_eq!(output.status.code(), Some(2), "{:?}", output);
        assert!(output.stdout.is_empty());
    }
    std::fs::remove_file(root.path().join("plumb.toml")).unwrap();
    std::fs::create_dir(root.path().join("plumb.toml")).unwrap();
    assert_eq!(inspect(root.path()).status.code(), Some(2));
}

#[cfg(unix)]
#[test]
fn linked() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source.toml");
    std::fs::write(&source, DECLARATION).unwrap();
    std::os::unix::fs::symlink(source, root.path().join("plumb.toml")).unwrap();
    assert_eq!(inspect(root.path()).status.code(), Some(2));
}
