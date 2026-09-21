use std::path::Path;

#[path = "policy/seat.rs"]
mod seat;
use seat::{policy, run};
use std::process::Command;

fn govern(root: &Path) {
    let status = Command::new("git")
        .args([
            "-C",
            root.to_str().expect("path should be utf8"),
            "init",
            "-q",
        ])
        .status()
        .expect("git should run");
    assert!(status.success(), "fixture should become a repository");
}

fn stock(home: &Path, policy: &str) {
    super::support::stock(
        &home.join("configurations"),
        &[("rules/policy.toml", policy)],
    );
}

fn source() -> String {
    super::support::rules(&["policy.toml"])[0].1.clone()
}

#[test]
fn blacklist() {
    let root = std::env::temp_dir().join("plumb-blacklist");
    std::fs::create_dir_all(root.join("apps/web")).expect("fixture should be made");
    std::fs::write(
        root.join("apps/web/package.json"),
        r#"{"dependencies":{"@stylexjs/stylex":"0.15.0"}}"#,
    )
    .expect("manifest should be written");
    let out = run(&root);
    std::fs::remove_dir_all(&root).expect("fixture should be swept");
    let manifest = Path::new("apps").join("web").join("package.json");
    let line = format!(
        "{} depends on blacklisted styling package @stylexjs/stylex",
        manifest.display()
    );
    assert!(out.contains(&line), "{out}");
}

#[test]
fn ban() {
    let root = std::env::temp_dir().join("plumb-policy-ban");
    std::fs::create_dir_all(root.join("apps/web/src/lib/components"))
        .expect("fixture should be made");
    std::fs::write(root.join("ectropy.toml"), "").expect("policy should be written");
    let out = run(&root);
    std::fs::remove_dir_all(&root).expect("fixture should be swept");
    assert!(
        out.contains("missing ectropy style ban apps/web/src/lib/components/**"),
        "{out}"
    );
}

#[test]
fn blind() {
    let root = std::env::temp_dir().join("plumb-blind");
    std::fs::create_dir_all(&root).expect("fixture should be made");
    govern(&root);
    std::fs::write(root.join("runseal.toml"), "").expect("profile should be made");
    std::fs::write(root.join("ectropy.toml"), "[limit\n").expect("policy should be written");
    let out = run(&root);
    std::fs::remove_dir_all(&root).expect("fixture should be swept");
    assert!(out.contains("blind:"), "{out}");
    assert!(out.contains("cannot read ectropy.toml"), "{out}");
}

#[test]
fn allowance() {
    let root = std::env::temp_dir().join("plumb-allowance");
    std::fs::create_dir_all(&root).expect("fixture should be made");
    govern(&root);
    let policy = r#"
[scan]
include = []
exclude = []
[module]
roots = []
[limit]
block = 4
path = 3
param = 4
markup = 8
file = 300
fanout = 10
[comment]
allow = false
[word]
single = true
[[grant]]
syntax = "style"
paths = ["packages/react-components/**"]
"#;
    std::fs::write(root.join("ectropy.toml"), policy).expect("policy should be written");
    let out = run(&root);
    std::fs::remove_dir_all(&root).expect("fixture should be swept");
    assert!(out.contains("true to the skeleton"), "{out}");
}

#[test]
fn reconciliation() {
    let root = std::env::temp_dir().join("plumb-policy-reconciliation");
    std::fs::create_dir_all(root.join("apps/web/src/lib/components"))
        .expect("fixture should be made");
    std::fs::create_dir_all(root.join("apps/web/tests")).expect("fixture should be made");
    std::fs::create_dir_all(root.join("skills/plumb")).expect("fixture should be made");
    std::fs::write(root.join("apps/web/vite.config.ts"), "").expect("fixture should be made");
    std::fs::write(
        root.join("ectropy.toml"),
        r#"
[scan]
include = ["old"]

[[grant]]
syntax = "style"
paths = ["packages/react-components/**"]

[[grant]]
syntax = "environment"
paths = ["apps/web/vite.config.ts"]

[[boundary]]
paths = ["apps/web/vite.config.ts"]
note = "vite config"
allow = ["path"]

[[vocabulary.term]]
name = "vite_config"
description = "fixture"
"#,
    )
    .expect("policy should be written");
    let output = policy(&root, true);
    assert!(output.status.success(), "{output:?}");
    let text = std::fs::read_to_string(root.join("ectropy.toml"))
        .expect("reconciled policy should be readable");
    let out = run(&root);
    std::fs::remove_dir_all(&root).expect("fixture should be swept");
    assert!(text.contains("apps/**/*.ts"), "{text}");
    assert!(text.contains("apps/web/vite.config.ts"), "{text}");
    assert!(text.contains("packages/react-components/**"), "{text}");
    assert!(text.contains("apps/web/src/lib/components/**"), "{text}");
    assert!(text.contains("skills/**/*.md"), "{text}");
    assert!(text.contains("skills/*"), "{text}");
    assert!(text.contains("name = \"vite_config\""), "{text}");
    assert!(!out.contains("missing ectropy"), "{out}");
    assert!(!out.contains("unexpected ectropy"), "{out}");
}

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
        &source().replace("docs/**/*.md", "notes/**/*.md"),
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
    let carried = source().replace("path = 3", "path = 4").replace(
        "roots = [\"apps/*/src\", \"apps/*/tests\"]",
        "roots = [\"apps/*\", \"apps/*/src\", \"apps/*/tests\"]",
    );
    stock(seat.path(), &carried);
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
    stock(
        seat.path(),
        "[limit]\nblock=4\nfanout=10\nfile=300\nmarkup=8\nparam=4\npath=4\n[comment]\nallow=false\n[word]\nsingle=true\n",
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
    assert!(!output.status.success(), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("rules/policy.toml must hold shape rows"),
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
