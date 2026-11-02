use super::{inspect, plant};
use std::path::Path;
use std::process::Command;

const FILES: [(&str, &str); 6] = [
    (
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/app\", \"crates/support\"]\nresolver = \"3\"\n",
    ),
    (
        "crates/app/Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    ),
    (
        "crates/app/src/lib.rs",
        "#[path = \"../../support/src/gate.rs\"]\nmod gate;\n",
    ),
    (
        "crates/support/Cargo.toml",
        "[package]\nname = \"support\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    ),
    ("crates/support/src/lib.rs", "mod gate;\n"),
    (
        "crates/support/src/gate.rs",
        "pub fn hold(file: &std::fs::File) { file.lock().unwrap(); }\n",
    ),
];

fn findings(text: &str) -> serde_json::Value {
    let result: serde_json::Value = serde_json::from_str(text).expect("inspection should be json");
    let held = result["findings"].as_array().expect("findings");
    assert_eq!(
        held.len(),
        1,
        "shared source should not become a second detached finding"
    );
    assert_eq!(held[0]["path"], "crates/support/src/gate.rs");
    assert_eq!(held[0]["call"], "std::fs::File::lock");
    assert!(!Path::new(held[0]["path"].as_str().expect("path")).is_absolute());
    result["findings"].clone()
}

#[test]
fn shared() {
    let first = plant(&FILES);
    let second = plant(&FILES);
    assert_eq!(
        findings(&inspect(first.path(), true)),
        findings(&inspect(second.path(), true))
    );
}

#[test]
fn relative() {
    let root = plant(&FILES);
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["lock", "inspect", ".", "--json"])
        .current_dir(root.path())
        .env("CARGO_HOME", root.path().join(".cargo"))
        .env("PLUMB_HOME", root.path().join(".plumb"))
        .output()
        .expect("plumb should run");
    assert!(output.status.success(), "{output:?}");
    findings(&String::from_utf8(output.stdout).expect("output should be utf8"));
}
