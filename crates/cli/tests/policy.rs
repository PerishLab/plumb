use std::path::Path;
use std::process::Command;

fn run(root: &Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["doctor", root.to_str().expect("path should be utf8")])
        .output()
        .expect("plumb should run");
    String::from_utf8_lossy(&output.stdout).to_string()
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
    let line = "apps/web/package.json depends on blacklisted styling package @stylexjs/stylex";
    assert!(out.contains(line), "{out}");
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
    std::fs::create_dir_all(root.join(".runseal/wrappers")).expect("fixture should be made");
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
    let policy = r#"
[scan]
include = []
exclude = []
[module]
roots = []
[limit]
block = 4
path = 4
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
