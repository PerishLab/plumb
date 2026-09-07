use super::super::world::{Fixture, run};
use super::marker::seeded;
use base64::Engine as _;
use serde::Serialize;
use sha2::{Digest as _, Sha256};
use std::path::Path;
use std::process::Command;

#[test]
fn unfinished() {
    let temp = tempfile::tempdir().expect("temp root");
    let bare = tempfile::tempdir().expect("bare root");
    let tools = temp.path().join("tools");
    std::fs::create_dir(&tools).expect("tool root");
    let fixture = Fixture {
        root: temp.path(),
        tools: &tools,
    };
    seeded(&fixture, bare.path());
    let tree = text(fixture.root, &["rev-parse", "HEAD^{tree}"]);
    let proof = proof(fixture.root, &tree);
    let message = format!("proved main before the next datum\n\nPlumb-Guard-Proof: {proof}");
    run(Command::new("git").arg("-C").arg(fixture.root).args([
        "commit",
        "--allow-empty",
        "-qm",
        &message,
    ]));
    run(Command::new("git").arg("-C").arg(fixture.root).args([
        "push",
        "-q",
        "origin",
        "HEAD:refs/heads/release/v1.2.1",
    ]));

    let stamped = fixture
        .command()
        .current_dir(fixture.root)
        .args(["release", "stamp", "--version", "v1.2.1-beta.1"])
        .output()
        .expect("plumb should run");
    assert!(!stamped.status.success());
    let error = String::from_utf8_lossy(&stamped.stderr);
    assert!(
        error.contains("finish version prepare before stamping"),
        "{error}"
    );
    let remote = run(Command::new("git").arg("-C").arg(fixture.root).args([
        "ls-remote",
        "--tags",
        "origin",
        "v1.2.1-beta.1",
    ]));
    assert!(remote.stdout.is_empty());
}

pub(super) fn proof(root: &Path, tree: &str) -> String {
    #[derive(Serialize)]
    struct Claim<'a> {
        schema: &'a str,
        repository: &'a str,
        tree: &'a str,
        plumb: &'a str,
        depot: &'a str,
        platform: &'a str,
        actions: &'a [plumb::guard::Action],
    }
    let remote = text(root, &["remote", "get-url", "origin"]);
    let path = remote.trim_end_matches('/').trim_end_matches(".git");
    let mut parts = path.split('/').rev();
    let repository = format!(
        "{}/{}",
        parts.nth(1).expect("owner"),
        path.split('/').next_back().expect("repository")
    );
    let actions = vec![plumb::guard::Action {
        name: "guard/test".into(),
        input: "0".repeat(64),
        world: "1".repeat(64),
    }];
    let plumb = plumb::version!("PLUMB").to_string();
    let depot = "2".repeat(64);
    let platform = plumb::config::platform();
    let claim = Claim {
        schema: plumb::guard::SCHEMA,
        repository: &repository,
        tree,
        plumb: &plumb,
        depot: &depot,
        platform: &platform,
        actions: &actions,
    };
    let digest = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&claim).expect("claim"))
    );
    let held = plumb::guard::Descriptor {
        schema: claim.schema.into(),
        repository,
        tree: tree.into(),
        plumb,
        depot,
        platform,
        actions,
        digest,
    };
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_vec(&held).expect("proof"))
}

fn text(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("git");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}
