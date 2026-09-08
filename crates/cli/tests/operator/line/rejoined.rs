use super::datum::lined;
use super::stable::{command, run};
use super::world::{Court, serve};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::env;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};

#[test]
fn late() {
    let fixture = tempfile::tempdir().expect("fixture");
    let bare = tempfile::tempdir().expect("bare");
    let root = fixture.path();
    let cut = root.join("cut");
    let (url, _) = serve(Court::Prepare(true, cut.clone()), 10);
    let origin = format!("{url}/test/probe.git");
    let head = lined(root, &origin, bare.path(), "release/v1.3.0");
    std::fs::write(&cut, &head).expect("cut");

    std::fs::write(root.join("stray"), "released, never rejoined\n").expect("stray");
    run(Command::new("git").args(["add", "-A"]).current_dir(root));
    run(Command::new("git")
        .args([
            "commit",
            "-q",
            "-m",
            "Carry work only the release line holds",
        ])
        .current_dir(root));
    run(Command::new("git")
        .args(["tag", "v1.2.0", "HEAD"])
        .current_dir(root));
    run(Command::new("git")
        .args(["update-ref", "refs/remotes/origin/main", &head])
        .current_dir(root));
    activate(root, "v1.2.0", &reference(root, "v1.2.0^{commit}"));

    let output = execute(root, &url, &["version", "prepare", "--version", "1.3.0"]);
    assert!(!output.status.success());
    let said = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(said.contains("stable v1.2.0 stands at"), "{said}");
    assert!(
        said.contains("plumb version rejoin --version v1.2.0"),
        "{said}"
    );
}

#[test]
fn refreshes() {
    let fixture = tempfile::tempdir().expect("fixture");
    let bare = tempfile::tempdir().expect("bare");
    let root = fixture.path();
    let cut = root.join("cut");
    let (url, _) = serve(Court::Prepare(true, cut.clone()), 3);
    let origin = format!("{url}/test/probe.git");
    let head = lined(root, &origin, bare.path(), "release/v1.3.0");
    std::fs::write(&cut, &head).expect("cut");
    run(Command::new("git")
        .args(["tag", "v1.2.0", &head])
        .current_dir(root));
    activate(root, "v1.2.0", &head);

    let tree = Command::new("git")
        .args(["rev-parse", &format!("{head}^{{tree}}")])
        .current_dir(root)
        .output()
        .expect("git tree");
    let tree = String::from_utf8_lossy(&tree.stdout).trim().to_string();
    let severed = Command::new("git")
        .args([
            "commit-tree",
            &tree,
            "-m",
            "Move main away from the stable point",
        ])
        .env("GIT_AUTHOR_NAME", "probe")
        .env("GIT_AUTHOR_EMAIL", "probe@test")
        .env("GIT_COMMITTER_NAME", "probe")
        .env("GIT_COMMITTER_EMAIL", "probe@test")
        .current_dir(bare.path())
        .output()
        .expect("git commit-tree");
    assert!(
        severed.status.success(),
        "{}",
        String::from_utf8_lossy(&severed.stderr)
    );
    let severed = String::from_utf8_lossy(&severed.stdout).trim().to_string();
    run(Command::new("git")
        .args(["update-ref", "refs/heads/main", &severed])
        .current_dir(bare.path()));

    let output = execute(root, &url, &["version", "prepare", "--version", "1.3.0"]);
    assert!(!output.status.success());
    let said = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(said.contains("which origin/main does not hold"), "{said}");
    let refreshed = Command::new("git")
        .args(["rev-parse", "origin/main"])
        .current_dir(root)
        .output()
        .expect("git rev-parse");
    assert_eq!(String::from_utf8_lossy(&refreshed.stdout).trim(), severed);
}

#[test]
fn freedom() {
    let fixture = tempfile::tempdir().expect("fixture");
    let bare = tempfile::tempdir().expect("bare");
    let root = fixture.path();
    let cut = root.join("cut");
    let (url, _) = serve(Court::Prepare(true, cut.clone()), 8);
    let origin = format!("{url}/test/probe.git");
    let head = lined(root, &origin, bare.path(), "release/v1.2.0");
    std::fs::write(&cut, &head).expect("cut");

    std::fs::write(root.join("failed"), "never activated\n").expect("failed marker");
    run(Command::new("git")
        .args(["add", "failed"])
        .current_dir(root));
    run(Command::new("git")
        .args(["commit", "-q", "-m", "Stand a failed release marker"])
        .current_dir(root));
    run(Command::new("git")
        .args(["tag", "v1.1.0", "HEAD"])
        .current_dir(root));
    run(Command::new("git")
        .args(["update-ref", "refs/remotes/origin/main", &head])
        .current_dir(root));
    let output = execute(root, &url, &["version", "prepare", "--version", "1.2.0"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn migrated() {
    let fixture = tempfile::tempdir().expect("fixture");
    let bare = tempfile::tempdir().expect("bare");
    let root = fixture.path();
    let cut = root.join("cut");
    let (url, _) = serve(Court::Prepare(true, cut.clone()), 9);
    let origin = format!("{url}/test/probe.git");
    std::fs::create_dir_all(root.join(".plumb/releases/v1.2.0")).expect("seat");
    std::fs::write(
        root.join(".plumb/releases/v1.2.0/datum.json"),
        "{\"schema\":1}\n",
    )
    .expect("an older Plumb's datum");
    let head = lined(root, &origin, bare.path(), "release/v1.2.0");
    std::fs::write(&cut, &head).expect("cut");
    let output = command(root, &["version", "prepare", "--version", "1.2.0"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let listed = Command::new("git")
        .args(["log", "--format=%H", "release/v1.2.0"])
        .current_dir(bare.path())
        .output()
        .expect("git");
    let recorded = String::from_utf8_lossy(&listed.stdout)
        .lines()
        .next()
        .expect("datum commit")
        .to_string();
    let shown = Command::new("git")
        .args(["show", "--name-only", "--format=", &recorded])
        .current_dir(bare.path())
        .output()
        .expect("git");
    let touched = String::from_utf8_lossy(&shown.stdout).to_string();
    assert!(!touched.contains("datum.toml"), "{touched}");
    assert!(
        plumb::datum::Git(bare.path())
            .at("v1.2.0", &recorded)
            .unwrap()
            .is_some()
    );
    assert!(
        touched.contains("datum.json"),
        "the recording commit drops what it swept, so freeze sees one seat commit: {touched}"
    );
}

fn activate(root: &Path, version: &str, commit: &str) {
    let authority = "https://releases.test";
    let url = format!("{authority}/v1/releases/stable/{version}/seal.json");
    let seal = json!({
        "schema": 1,
        "product": "probe",
        "channel": "stable",
        "releaseVersion": version,
        "commit": commit,
        "url": url,
        "generator": {"version": version, "template": "test", "origin": {"kind": "stable"}},
        "artifacts": {},
        "managers": {}
    });
    let bytes = serde_json::to_vec(&seal).expect("seal");
    std::fs::write(root.join("seal.json"), &bytes).expect("seal record");
    let pointer = json!({
        "schema": 1,
        "product": "probe",
        "channel": "stable",
        "releaseVersion": version,
        "commit": commit,
        "seal": {
            "name": "seal.json",
            "mime": "application/json",
            "sha256": format!("{:x}", Sha256::digest(&bytes)),
            "size": bytes.len(),
            "url": url
        },
        "managers": {}
    });
    std::fs::write(
        root.join("pointer.json"),
        serde_json::to_vec(&pointer).expect("pointer"),
    )
    .expect("pointer record");
}

fn execute(root: &Path, server: &str, args: &[&str]) -> Output {
    let bin = root.join(".git/court-bin");
    std::fs::create_dir_all(&bin).expect("court bin");
    let curl = bin.join("curl");
    std::fs::write(
        &curl,
        format!(
            r#"#!/bin/bash
args=("$@")
for i in "${{!args[@]}}"; do
  args[$i]="${{args[$i]//https:\/\/releases.test/{server}}}"
done
exec /usr/bin/curl "${{args[@]}}"
"#
        ),
    )
    .expect("curl shim");
    std::fs::set_permissions(&curl, std::fs::Permissions::from_mode(0o755)).expect("curl mode");
    let path = env::join_paths(
        std::iter::once(bin).chain(env::split_paths(&env::var_os("PATH").unwrap_or_default())),
    )
    .expect("PATH");
    super::command::plumb(root, args)
        .env("PATH", path)
        .env("HARNESS_RUN_TIMEOUT_MS", "1000")
        .output()
        .expect("plumb")
}

fn reference(root: &Path, name: &str) -> String {
    let output = Command::new("git")
        .args(["rev-parse", name])
        .current_dir(root)
        .output()
        .expect("git rev-parse");
    assert!(output.status.success());
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}
