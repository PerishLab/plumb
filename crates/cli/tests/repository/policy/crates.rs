use super::{LIMIT, SCAN, SHAPE, policy, stock};
use std::process::Command;

#[test]
fn rust() {
    let scratch = tempfile::tempdir().expect("fixture parent should be made");
    let root = scratch.path().join("plumb-policy-crates-rs");
    std::fs::create_dir_all(root.join("crates/cli/src")).expect("fixture should be made");
    std::fs::write(root.join("crates/cli/src/lib.rs"), "").expect("fixture should be made");
    std::fs::write(root.join("ectropy.toml"), "").expect("policy should be written");
    let output = policy(&root, true);
    assert!(output.status.success(), "{output:?}");
    let text = std::fs::read_to_string(root.join("ectropy.toml"))
        .expect("reconciled policy should be readable");
    assert!(text.contains("crates/**/*.rs"), "{text}");
    assert!(!text.contains("crates/**/*.ts"), "{text}");
    assert!(!text.contains("crates/**/*.tsx"), "{text}");
}

#[test]
fn typescript() {
    let scratch = tempfile::tempdir().expect("fixture parent should be made");
    let root = scratch.path().join("plumb-policy-crates-ts");
    std::fs::create_dir_all(root.join("crates/cli/scripts")).expect("fixture should be made");
    std::fs::write(root.join("crates/cli/scripts/act.ts"), "").expect("fixture should be made");
    std::fs::write(root.join("ectropy.toml"), "").expect("policy should be written");
    let output = policy(&root, true);
    assert!(output.status.success(), "{output:?}");
    let text = std::fs::read_to_string(root.join("ectropy.toml"))
        .expect("reconciled policy should be readable");
    assert!(text.contains("crates/**/*.rs"), "{text}");
    assert!(text.contains("crates/**/*.ts"), "{text}");
    assert!(text.contains("crates/**/*.tsx"), "{text}");
    assert!(text.contains("**/node_modules/**"), "{text}");
}

#[test]
fn tsx() {
    let scratch = tempfile::tempdir().expect("fixture parent should be made");
    let root = scratch.path().join("plumb-policy-crates-tsx");
    std::fs::create_dir_all(root.join("crates/cli/src")).expect("fixture should be made");
    std::fs::write(root.join("crates/cli/src/App.tsx"), "").expect("fixture should be made");
    std::fs::write(root.join("ectropy.toml"), "").expect("policy should be written");
    let home = root.join(".plumb-test-home");
    stock(&home, LIMIT, SCAN, SHAPE);
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args([
            "policy",
            root.to_str().expect("path should be utf8"),
            "--write",
        ])
        .env("PLUMB_HOME", &home)
        .output()
        .expect("plumb should run");
    assert!(output.status.success(), "{output:?}");
    let text = std::fs::read_to_string(root.join("ectropy.toml"))
        .expect("reconciled policy should be readable");
    assert!(text.contains("crates/**/*.ts"), "{text}");
    assert!(text.contains("crates/**/*.tsx"), "{text}");
}

fn reconciled(name: &str, files: &[&str]) -> String {
    let scratch = tempfile::tempdir().expect("fixture parent should be made");
    let root = scratch.path().join(name);
    for file in files {
        let path = root.join(file);
        std::fs::create_dir_all(path.parent().expect("fixture file has a parent"))
            .expect("fixture should be made");
        std::fs::write(path, "").expect("fixture should be made");
    }
    std::fs::write(root.join("ectropy.toml"), "").expect("policy should be written");
    let output = policy(&root, true);
    assert!(output.status.success(), "{output:?}");
    std::fs::read_to_string(root.join("ectropy.toml"))
        .expect("reconciled policy should be readable")
}

#[test]
fn derived() {
    let text = reconciled(
        "plumb-policy-crates-derived",
        &[
            "crates/cli/src/lib.rs",
            "crates/cli/scripts/act.ts",
            "crates/cli/scripts/lib/cmd.ts",
        ],
    );
    assert!(text.contains("\"crates/cli/scripts\""), "{text}");
    assert!(!text.contains("\"crates/cli/scripts/lib\""), "{text}");
}

#[test]
fn seated() {
    let text = reconciled(
        "plumb-policy-crates-seated",
        &["crates/cli/src/main.rs", "crates/cli/src/view.ts"],
    );
    assert!(text.contains("crates/**/*.ts"), "{text}");
    assert!(!text.contains("\"crates/cli/src\""), "{text}");
}

#[test]
fn underived() {
    let text = reconciled(
        "plumb-policy-crates-underived",
        &["crates/cli/src/lib.rs", "crates/cli/scripts/run.sh"],
    );
    assert!(!text.contains("crates/cli/scripts"), "{text}");
}
