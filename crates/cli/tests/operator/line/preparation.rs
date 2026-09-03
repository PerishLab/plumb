use super::command::support;
use super::datum::lined;
use super::stable::{command, run};
use super::world::{Court, serve};
use base64::Engine as _;
use serde::Serialize;
use sha2::{Digest as _, Sha256};
use std::env;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn controller() {
    let fixture = tempfile::tempdir().expect("fixture");
    let bare = tempfile::tempdir().expect("bare");
    let home = support::depot(&[]);
    let root = fixture.path();
    let cut = root.join("cut");
    let (url, _) = serve(Court::Prepare(true, cut.clone()), 7);
    let origin = format!("{url}/test/probe.git");
    let head = lined(root, &origin, bare.path(), "release/v1.2.0");
    std::fs::write(&cut, &head).expect("cut");

    let hooks = root.join(".git/hooks");
    std::fs::create_dir_all(&hooks).expect("hooks");
    let hook = hooks.join("pre-commit");
    std::fs::write(&hook, "#!/bin/sh\nexec plumb --version\n").expect("hook");
    let mut mode = std::fs::metadata(&hook)
        .expect("hook metadata")
        .permissions();
    mode.set_mode(0o755);
    std::fs::set_permissions(&hook, mode).expect("executable hook");

    let stale = root.join("stale-bin");
    std::fs::create_dir(&stale).expect("stale bin");
    let binary = stale.join("plumb");
    std::fs::write(&binary, "#!/bin/sh\nexit 93\n").expect("stale plumb");
    let mut mode = std::fs::metadata(&binary)
        .expect("stale metadata")
        .permissions();
    mode.set_mode(0o755);
    std::fs::set_permissions(&binary, mode).expect("executable stale plumb");
    let path = env::join_paths(
        std::iter::once(stale).chain(env::split_paths(&env::var_os("PATH").unwrap_or_default())),
    )
    .expect("PATH");

    let output = super::command::plumb(root, &["version", "prepare", "--version", "1.2.0"])
        .env("PATH", path)
        .env("PLUMB_HOME", home.path())
        .env_remove("PLUMB_GUARD_CONFIGURATION")
        .output()
        .expect("plumb");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

pub fn provenance(root: &Path, cut: PathBuf) {
    guarded(root, &cut);
    let (forge, _) = serve(Court::Freeze(cut.clone()), 1);
    run(Command::new("git")
        .args(["config", "plumb.test-forgejo-url", &forge])
        .current_dir(root));
    let valid = command(
        root,
        &["version", "freeze", "--version", "1.2.0", "--dry-run"],
    );
    let valid = String::from_utf8_lossy(&valid.stderr);
    assert!(
        !valid.contains("without cherry-pick -x provenance"),
        "{valid}"
    );

    run(Command::new("git")
        .args(["fetch", "origin", "release/v1.2.0"])
        .current_dir(root));
    run(Command::new("git")
        .args(["switch", "--detach", "origin/release/v1.2.0"])
        .current_dir(root));
    std::fs::write(root.join("unowned.txt"), "drift\n").expect("drift");
    run(Command::new("git").args(["add", "-A"]).current_dir(root));
    run(Command::new("git")
        .args(["commit", "-q", "-m", "Prepare v1.2.0"])
        .current_dir(root));
    run(Command::new("git")
        .args(["push", "origin", "HEAD:refs/heads/release/v1.2.0"])
        .current_dir(root));
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()
        .expect("git");
    assert!(output.status.success());
    std::fs::write(&cut, String::from_utf8_lossy(&output.stdout).trim()).expect("forged cut");
    let (forge, _) = serve(Court::Freeze(cut), 1);
    run(Command::new("git")
        .args(["config", "plumb.test-forgejo-url", &forge])
        .current_dir(root));
    let refused = command(
        root,
        &["version", "freeze", "--version", "1.2.0", "--dry-run"],
    );
    assert!(String::from_utf8_lossy(&refused.stderr).contains("without cherry-pick -x provenance"));
}

fn guarded(root: &Path, cut: &Path) {
    run(Command::new("git")
        .args(["fetch", "origin", "release/v1.2.0"])
        .current_dir(root));
    let head = text(root, &["rev-parse", "origin/release/v1.2.0"]);
    let prepared = text(root, &["rev-parse", &format!("{head}^")]);
    let base = text(root, &["rev-parse", &format!("{prepared}^")]);
    let tree = text(root, &["rev-parse", &format!("{prepared}^{{tree}}")]);
    let prepared = text(
        root,
        &[
            "commit-tree",
            &tree,
            "-p",
            &base,
            "-m",
            "Prepare v1.2.0\n\nPlumb-Guard-Proof: exact",
        ],
    );
    let tree = text(root, &["rev-parse", &format!("{head}^{{tree}}")]);
    let head = text(
        root,
        &[
            "commit-tree",
            &tree,
            "-p",
            &prepared,
            "-m",
            "Record the datum v1.2.0 judges against",
        ],
    );
    let remote = text(root, &["remote", "get-url", "origin"]);
    let path = remote.trim_end_matches('/').trim_end_matches(".git");
    let mut parts = path.split('/').rev();
    let repository = format!(
        "{}/{}",
        parts.nth(1).expect("owner"),
        path.split('/').next_back().expect("repository")
    );
    let proof = proof(&repository, &tree);
    let head = text(
        root,
        &[
            "commit-tree",
            &tree,
            "-p",
            &head,
            "-m",
            &format!("Refresh the release proof\n\nPlumb-Guard-Proof: {proof}"),
        ],
    );
    run(Command::new("git")
        .args([
            "push",
            "--force",
            "origin",
            &format!("{head}:refs/heads/release/v1.2.0"),
        ])
        .current_dir(root));
    run(Command::new("git")
        .args(["update-ref", "refs/remotes/origin/release/v1.2.0", &head])
        .current_dir(root));
    std::fs::write(cut, head).expect("guarded cut");
}

fn proof(repository: &str, tree: &str) -> String {
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
    let actions = vec![plumb::guard::Action {
        name: "guard/test".into(),
        input: "0".repeat(64),
        world: "1".repeat(64),
    }];
    let depot = "2".repeat(64);
    let claim = Claim {
        schema: plumb::guard::SCHEMA,
        repository,
        tree,
        plumb: "v0.0.0",
        depot: &depot,
        platform: "test",
        actions: &actions,
    };
    let digest = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&claim).expect("claim"))
    );
    let held = plumb::guard::Descriptor {
        schema: claim.schema.into(),
        repository: claim.repository.into(),
        tree: tree.into(),
        plumb: claim.plumb.into(),
        depot: depot.clone(),
        platform: claim.platform.into(),
        actions: actions.clone(),
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
