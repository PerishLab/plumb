use super::{LIMIT, SCAN, SHAPE, govern, policy, stock};
use std::process::Command;

#[test]
fn svelte() {
    let root = std::env::temp_dir().join("plumb-policy-svelte");
    std::fs::create_dir_all(root.join("apps/docs/src")).expect("fixture should be made");
    std::fs::create_dir_all(root.join("packages/design/src")).expect("fixture should be made");
    std::fs::write(root.join("apps/docs/src/App.svelte"), "").expect("fixture should be made");
    std::fs::write(root.join("packages/design/src/Button.svelte"), "")
        .expect("fixture should be made");
    std::fs::write(root.join("ectropy.toml"), "").expect("policy should be written");
    let output = policy(&root, true);
    assert!(output.status.success(), "{output:?}");
    let text = std::fs::read_to_string(root.join("ectropy.toml"))
        .expect("reconciled policy should be readable");
    std::fs::remove_dir_all(&root).expect("fixture should be swept");
    assert!(text.contains("apps/**/*.svelte"), "{text}");
    assert!(text.contains("packages/**/*.svelte"), "{text}");
    assert!(text.contains("**/.svelte-kit/**"), "{text}");
    assert!(!text.contains("apps/**/*.tsx"), "{text}");
    assert!(!text.contains("packages/**/*.tsx"), "{text}");
}

#[test]
fn carriage() {
    let root = tempfile::tempdir().expect("fixture");
    let seat = tempfile::tempdir().expect("seat");
    std::fs::create_dir_all(root.path().join("docs")).expect("fixture should be made");
    std::fs::write(root.path().join("ectropy.toml"), "").expect("policy should be written");
    stock(
        seat.path(),
        LIMIT,
        &SCAN.replace("docs/**/*.md", "notes/**/*.md"),
        SHAPE,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args([
            "policy",
            root.path().to_str().expect("path should be utf8"),
            "--write",
        ])
        .env("PLUMB_HOME", seat.path())
        .output()
        .expect("plumb should run");
    assert!(output.status.success(), "{output:?}");
    let text = std::fs::read_to_string(root.path().join("ectropy.toml"))
        .expect("reconciled policy should be readable");
    assert!(text.contains("notes/**/*.md"), "{text}");
    assert!(!text.contains("docs/**/*.md"), "{text}");
}

#[test]
fn transition() {
    let root = tempfile::tempdir().expect("fixture");
    let seat = tempfile::tempdir().expect("seat");
    std::fs::create_dir_all(root.path().join("apps/web")).expect("web seat");
    govern(root.path());
    let rendered = policy(root.path(), true);
    assert!(rendered.status.success(), "{rendered:?}");
    let carried = SCAN.replace(
        "roots = [\"apps/*/src\", \"apps/*/tests\"]",
        "roots = [\"apps/*\", \"apps/*/src\", \"apps/*/tests\"]",
    );
    stock(
        seat.path(),
        &LIMIT.replace("path = 3", "path = 4"),
        &carried,
        SHAPE,
    );
    let judged = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["doctor", root.path().to_str().expect("path should be utf8")])
        .env("PLUMB_HOME", seat.path())
        .output()
        .expect("plumb should run");
    assert!(
        String::from_utf8_lossy(&judged.stdout).contains("ectropy limit path must be 4"),
        "{}",
        String::from_utf8_lossy(&judged.stdout)
    );
    assert!(!judged.status.success());
}

#[test]
fn refusal() {
    let root = tempfile::tempdir().expect("fixture");
    let seat = tempfile::tempdir().expect("seat");
    std::fs::create_dir_all(root.path().join("docs")).expect("fixture should be made");
    std::fs::write(root.path().join("ectropy.toml"), "").expect("policy should be written");
    stock(seat.path(), LIMIT, SCAN, "");
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args([
            "policy",
            root.path().to_str().expect("path should be utf8"),
            "--write",
        ])
        .env("PLUMB_HOME", seat.path())
        .output()
        .expect("plumb should run");
    assert!(!output.status.success(), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("rules/suites/shape.toml must hold shape rows"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = std::fs::read_to_string(root.path().join("ectropy.toml"))
        .expect("reconciled policy should be readable");
    assert!(text.is_empty(), "{text}");
}

#[test]
fn malformed() {
    let root = std::env::temp_dir().join("plumb-policy-malformed");
    std::fs::create_dir_all(&root).expect("fixture should be made");
    govern(&root);
    let path = root.join("ectropy.toml");
    std::fs::write(&path, "[limit\n").expect("policy should be written");
    let output = policy(&root, true);
    let text = std::fs::read_to_string(&path).expect("policy should remain readable");
    std::fs::remove_dir_all(&root).expect("fixture should be swept");
    assert!(!output.status.success(), "{output:?}");
    assert_eq!(text, "[limit\n");
}
