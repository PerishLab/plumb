use std::path::Path;
use std::process::Command;

pub(super) const POLICY: &str = r#"
[limit]
block = 4
fanout = 10
file = 300
markup = 8
param = 4
path = 3

[comment]
allow = false

[word]
single = true

[[shape]]
when = ["docs"]
include = ["docs/**/*.md"]
roots = ["docs"]

[[shape]]
when = ["skills"]
include = ["skills/**/*.md"]
roots = ["skills/*"]

[[shape]]
when = ["apps/web/src/lib/components"]
bans = ["apps/web/src/lib/components/**"]

[[web]]
seat = "apps"
include = ["apps/**/*.ts"]
roots = ["apps/*/src", "apps/*/tests"]
svelte-include = ["apps/**/*.svelte"]
svelte-exclude = ["**/.svelte-kit/**"]
tsx-include = ["apps/**/*.tsx"]

[[web]]
seat = "packages"
include = ["packages/**/*.ts"]
roots = ["packages/*/src"]
svelte-include = ["packages/**/*.svelte"]
svelte-exclude = ["**/.svelte-kit/**"]
tsx-include = ["packages/**/*.tsx"]
"#;
const DEPS: &str = "blacklist = [\"@stylexjs/stylex\"]\n\n[stable.cargo]\nregistry = \"fixture\"\nindex = \"sparse+https://registry.invalid/\"\n";

pub(super) fn govern(root: &Path) {
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

pub(super) fn run(root: &Path) -> String {
    let home = root.join(".plumb-test-home");
    stock(&home, POLICY);
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["doctor", root.to_str().expect("path should be utf8")])
        .env("PLUMB_HOME", home)
        .output()
        .expect("plumb should run");
    String::from_utf8_lossy(&output.stdout).to_string()
}

pub(super) fn policy(root: &Path, write: bool) -> std::process::Output {
    let home = root.join(".plumb-test-home");
    stock(&home, POLICY);
    let mut command = Command::new(env!("CARGO_BIN_EXE_plumb"));
    command.args(["policy", root.to_str().expect("path should be utf8")]);
    command.env("PLUMB_HOME", home);
    if write {
        command.arg("--write");
    }
    command.output().expect("plumb should run")
}

pub(super) fn stock(home: &Path, policy: &str) {
    super::support::stock(
        &home.join("configurations"),
        &[("rules/policy.toml", policy), ("rules/deps.toml", DEPS)],
    );
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

mod web;
