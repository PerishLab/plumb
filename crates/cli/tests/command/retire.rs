use std::path::{Path, PathBuf};
use std::process::Command;

use super::configuration::support;

const ZONE: &str = "0123456789abcdef0123456789abcdef";
const BASE: &str = "[release]\nproduct = \"foo\"\nauthority = \"https://releases.foo.example\"\nbinaries = [\"foo\"]\ntargets = [\"x86_64-unknown-linux-gnu\"]\n";

fn seat(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(name);
    if root.exists() {
        std::fs::remove_dir_all(&root).expect("old fixture should be swept");
    }
    std::fs::create_dir_all(&root).expect("fixture should be made");
    for args in [
        vec!["init", "-q"],
        vec![
            "remote",
            "add",
            "origin",
            "ssh://git@git.example/perish/foo.git",
        ],
    ] {
        let done = Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(args)
            .status()
            .expect("git should run");
        assert!(done.success(), "fixture should become a repository");
    }
    root
}

fn declare(root: &Path, retire: &str) {
    std::fs::write(root.join("plumb.toml"), format!("{BASE}{retire}"))
        .expect("manifest should be written");
}

fn run(root: &Path, args: &[&str]) -> (bool, String) {
    let home = support::home();
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .env("PLUMB_HOME", home.path())
        .arg("retire")
        .arg("--root")
        .arg(root)
        .args(args)
        .env_remove("PLUMB_RETIRE_ACCOUNT")
        .env_remove("PLUMB_RETIRE_TOKEN")
        .output()
        .expect("plumb should run");
    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    (output.status.success(), text)
}

fn seeded() -> String {
    format!("\n[release.retire]\nbucket = \"perish-foo-releases\"\nzone = \"{ZONE}\"\n")
}

#[test]
fn undeclared() {
    let root = seat("plumb-retire-undeclared");
    declare(&root, "");
    let (ok, held) = run(&root, &[]);
    assert!(!ok, "{held}");
    assert!(held.contains("declares no release.retire seat"), "{held}");
    std::fs::remove_dir_all(&root).expect("fixture should be swept");
}

#[test]
fn dry() {
    let root = seat("plumb-retire-dry");
    declare(&root, &seeded());
    let (ok, held) = run(&root, &[]);
    assert!(ok, "{held}");
    assert!(held.contains("bucket: perish-foo-releases"), "{held}");
    assert!(held.contains("domain: releases.foo.example"), "{held}");
    assert!(held.contains("repo: perish/foo"), "{held}");
    assert!(
        held.contains("writer token: w:perish-foo-releases"),
        "{held}"
    );
    assert!(
        held.contains("no credentials read and no state changed"),
        "{held}"
    );
    std::fs::remove_dir_all(&root).expect("fixture should be swept");
}

#[test]
fn unconfirmed() {
    let root = seat("plumb-retire-unconfirmed");
    declare(&root, &seeded());
    let (ok, held) = run(&root, &["--execute"]);
    assert!(!ok, "{held}");
    assert!(
        held.contains("--confirm-repo must exactly equal perish/foo"),
        "{held}"
    );

    let (bucket, seen) = run(
        &root,
        &[
            "--execute",
            "--confirm-repo",
            "perish/foo",
            "--confirm-bucket",
            "wrong",
        ],
    );
    assert!(!bucket, "{seen}");
    assert!(
        seen.contains("--confirm-bucket must exactly equal"),
        "{seen}"
    );

    let (domain, told) = run(
        &root,
        &[
            "--execute",
            "--confirm-repo",
            "perish/foo",
            "--confirm-bucket",
            "perish-foo-releases",
            "--confirm-domain",
            "releases.foo.example.evil",
        ],
    );
    assert!(!domain, "{told}");
    assert!(
        told.contains("--confirm-domain must exactly equal"),
        "{told}"
    );
    std::fs::remove_dir_all(&root).expect("fixture should be swept");
}

#[test]
fn credentialless() {
    let root = seat("plumb-retire-credentialless");
    declare(&root, &seeded());
    let (ok, held) = run(
        &root,
        &[
            "--execute",
            "--confirm-repo",
            "perish/foo",
            "--confirm-bucket",
            "perish-foo-releases",
            "--confirm-domain",
            "releases.foo.example",
        ],
    );
    assert!(!ok, "{held}");
    assert!(held.contains("missing PLUMB_RETIRE_ACCOUNT"), "{held}");
    std::fs::remove_dir_all(&root).expect("fixture should be swept");
}

#[test]
fn misdeclared() {
    let root = seat("plumb-retire-misdeclared");
    declare(
        &root,
        &format!("\n[release.retire]\nbucket = \"Perish_Foo\"\nzone = \"{ZONE}\"\n"),
    );
    let (ok, held) = run(&root, &[]);
    assert!(!ok, "{held}");
    assert!(held.contains("invalid retirement bucket"), "{held}");

    declare(
        &root,
        "\n[release.retire]\nbucket = \"perish-foo-releases\"\nzone = \"beef\"\n",
    );
    let (zoned, seen) = run(&root, &[]);
    assert!(!zoned, "{seen}");
    assert!(seen.contains("invalid retirement zone"), "{seen}");
    std::fs::remove_dir_all(&root).expect("fixture should be swept");
}
