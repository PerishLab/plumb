use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

fn run(home: &Path, key: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_plumb"))
        .env("PLUMB_HOME", home)
        .args(["metadata", key])
        .output()
        .expect("command")
}

fn configure(home: &Path, text: &str) {
    std::fs::write(home.join("plumb.toml"), text).expect("configuration");
}

fn records(home: &Path) -> Vec<Value> {
    let mut records = Vec::new();
    for entry in std::fs::read_dir(home.join("state/locus")).expect("buffer") {
        let path = entry.expect("entry").path();
        let name = path.file_name().expect("name").to_string_lossy();
        if name == "active.jsonl" || name.starts_with("sealed-") {
            let content = std::fs::read_to_string(path).expect("records");
            records.extend(
                content
                    .lines()
                    .map(|line| serde_json::from_str::<Value>(line).expect("atom")),
            );
        }
    }
    records
}

#[test]
fn disabled() {
    let home = tempfile::tempdir().expect("home");
    let absent = run(home.path(), "node.version");
    configure(home.path(), "[locus]\nenabled = false\n");
    let disabled = run(home.path(), "node.version");
    assert_eq!(absent.status.code(), disabled.status.code());
    assert_eq!(absent.stdout, disabled.stdout);
    assert_eq!(absent.stderr, disabled.stderr);
    assert!(!home.path().join("state/locus").exists());
}

#[test]
fn buffered() {
    let home = tempfile::tempdir().expect("home");
    configure(
        home.path(),
        "[locus]\nenabled = true\nendpoint = 'http://127.0.0.1:0'\n",
    );
    for (key, code) in [("node.version", 0), ("unknown", 2)] {
        let result = run(home.path(), key);
        assert_eq!(result.status.code(), Some(code));
    }
    let atoms = records(home.path());
    assert_eq!(atoms.len(), 10);
    assert!(atoms.iter().all(|atom| atom["producer"] == "plumb"));
    assert!(
        atoms
            .iter()
            .all(|atom| atom["context"]["plumb.command"] == "metadata")
    );
    for code in [0, 2] {
        assert!(atoms.iter().any(
            |atom| atom["payload"]["event"] == "cli.finish" && atom["payload"]["code"] == code
        ));
    }
    for edge in ["enter", "return"] {
        assert_eq!(
            atoms
                .iter()
                .filter(|atom| atom["source"]["edge"] == edge)
                .count(),
            2
        );
    }
}

#[test]
fn isolated() {
    let home = tempfile::tempdir().expect("home");
    let baseline = run(home.path(), "node.version");
    for config in [
        "[locus]\nenabled = true\nendpoint = 'http://127.0.0.1:0'\n",
        "[locus]\nenabled = 'malformed'\n",
        "[locus]\nenabled = true\nendpoint = ''\n",
    ] {
        configure(home.path(), config);
        let result = run(home.path(), "node.version");
        assert_eq!(result.status.code(), baseline.status.code());
        assert_eq!(result.stdout, baseline.stdout);
        assert!(!result.stderr.is_empty(), "diagnostic remains visible");
    }
}
