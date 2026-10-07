use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

const COOKBOOK: &str = "see: plumb cookbook env.toolchain-domain";

const ALIGNED: [(&str, &str); 4] = [
    ("cargo", "echo 'cargo 1.96.1 (ea2d97820 2026-06-26)'"),
    ("rustc", "echo 'rustc 1.96.1 (31fca3adb 2026-06-26)'"),
    ("node", "echo v24.18.0"),
    ("pnpm", "echo 11.13.0"),
];

fn stub(dir: &Path, tool: &str, body: &str) {
    let path = dir.join(tool);
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("stub should be written");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
        .expect("stub should be executable");
}

fn doctor(root: &Path, tools: &Path) -> String {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let mut paths = vec![tools.to_path_buf()];
    paths.extend(std::env::split_paths(&path));
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .env("PLUMB_HOME", super::support::home().keep())
        .env(
            "PATH",
            std::env::join_paths(paths).expect("path should join"),
        )
        .args(["doctor", root.to_str().expect("path should be utf8")])
        .output()
        .expect("plumb should run");
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn specimen(manifests: &[&str]) -> (tempfile::TempDir, tempfile::TempDir) {
    let root = tempfile::tempdir().expect("root");
    for name in manifests {
        std::fs::write(root.path().join(name), "{}\n").expect("manifest should be written");
    }
    let tools = tempfile::tempdir().expect("tools");
    for (tool, body) in ALIGNED {
        stub(tools.path(), tool, body);
    }
    (root, tools)
}

#[test]
fn aligned() {
    let (root, tools) = specimen(&["Cargo.toml", "package.json"]);
    let out = doctor(root.path(), tools.path());
    assert!(!out.contains(COOKBOOK), "{out}");
}

#[test]
fn diverged() {
    let (root, tools) = specimen(&["Cargo.toml", "package.json"]);
    stub(
        tools.path(),
        "rustc",
        "echo 'rustc 1.95.0 (abc 2026-05-01)'",
    );
    stub(tools.path(), "pnpm", "echo 11.13.1");
    stub(
        tools.path(),
        "cargo",
        "echo 'error: rustup could not choose a version of cargo to run' >&2; exit 1",
    );
    let out = doctor(root.path(), tools.path());
    for line in [
        "rustc reports 1.95.0, the domain runs 1.96.1",
        "pnpm reports 11.13.1, the domain runs 11.13.0",
        "cargo does not run (error: rustup could not choose a version of cargo to run), the domain runs 1.96.1",
    ] {
        assert!(
            out.contains(&format!("{line}; {COOKBOOK}")),
            "{line}: {out}"
        );
    }
    assert!(!out.contains("node reports"), "{out}");
}

#[test]
fn unused() {
    let (root, tools) = specimen(&[]);
    for tool in ["cargo", "rustc", "node", "pnpm"] {
        stub(tools.path(), tool, "echo 0.0.1");
    }
    let out = doctor(root.path(), tools.path());
    assert!(!out.contains(COOKBOOK), "{out}");
}
