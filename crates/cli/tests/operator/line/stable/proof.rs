use super::super::datum::lined;
use super::super::world::{Court, serve};
use super::{command, run};
use std::path::Path;
use std::process::Command;

#[test]
fn refresh() {
    let fixture = tempfile::tempdir().expect("fixture");
    let bare = tempfile::tempdir().expect("bare");
    let root = fixture.path();
    let cut = root.join("cut");
    let (url, _) = serve(Court::Prepare(true, cut.clone()), 18);
    let origin = format!("{url}/test/probe.git");
    let head = lined(root, &origin, bare.path(), "release/v1.2.0");
    std::fs::write(&cut, &head).expect("cut");
    let prepared = command(root, &["version", "prepare", "--version", "1.2.0"]);
    assert!(
        prepared.status.success(),
        "{}",
        String::from_utf8_lossy(&prepared.stderr)
    );

    run(Command::new("git")
        .args(["fetch", "origin", "release/v1.2.0"])
        .current_dir(root));
    let standing = show(root, "--format=%H --no-patch origin/release/v1.2.0")
        .trim()
        .to_string();
    let tree = show(root, &format!("--format=%T --no-patch {standing}"))
        .trim()
        .to_string();
    let stale = Command::new("git")
        .args([
            "commit-tree",
            &tree,
            "-p",
            &standing,
            "-m",
            "Stale release proof\n\nPlumb-Guard-Proof: stale",
        ])
        .current_dir(root)
        .output()
        .expect("git commit-tree");
    assert!(
        stale.status.success(),
        "{}",
        String::from_utf8_lossy(&stale.stderr)
    );
    let stale = String::from_utf8_lossy(&stale.stdout).trim().to_string();
    run(Command::new("git")
        .args([
            "push",
            "origin",
            &format!("{stale}:refs/heads/release/v1.2.0"),
        ])
        .current_dir(root));
    std::fs::write(&cut, &stale).expect("stale cut");
    hooks(root);

    let home = super::super::command::support::guard(&[], plumb::version!("PLUMB"));
    let refreshed = isolated(root, home.path());
    assert!(
        refreshed.status.success(),
        "{}",
        String::from_utf8_lossy(&refreshed.stderr)
    );
    let report = String::from_utf8_lossy(&refreshed.stdout);
    assert!(report.contains("refreshed the release proof"), "{report}");
    let head = show(bare.path(), "--format=%H --no-patch release/v1.2.0")
        .trim()
        .to_string();
    assert_ne!(head, stale, "refresh must advance the release line");
    assert_eq!(
        tree,
        show(bare.path(), &format!("--format=%T --no-patch {head}")).trim(),
        "refresh must preserve the release tree"
    );
    assert!(
        show(bare.path(), &format!("--format=%B --no-patch {head}")).contains("Plumb-Guard-Proof:"),
        "refreshed head must carry a proof"
    );
    plumb::guard::commit(root, &head).expect("refreshed proof must be valid");
}

#[test]
fn datum() {
    let fixture = tempfile::tempdir().expect("fixture");
    let bare = tempfile::tempdir().expect("bare");
    let root = fixture.path();
    let cut = root.join("cut");
    let (url, _) = serve(Court::Prepare(true, cut.clone()), 7);
    let origin = format!("{url}/test/probe.git");
    let head = lined(root, &origin, bare.path(), "release/v1.2.0");
    let tree = show(root, "--format=%T --no-patch HEAD").trim().to_string();
    let stale = Command::new("git")
        .args([
            "commit-tree",
            &tree,
            "-p",
            &head,
            "-m",
            "Stale release proof\n\nPlumb-Guard-Proof: stale",
        ])
        .current_dir(root)
        .output()
        .expect("git commit-tree");
    assert!(
        stale.status.success(),
        "{}",
        String::from_utf8_lossy(&stale.stderr)
    );
    let stale = String::from_utf8_lossy(&stale.stdout).trim().to_string();
    run(Command::new("git")
        .args([
            "push",
            "origin",
            &format!("{stale}:refs/heads/release/v1.2.0"),
        ])
        .current_dir(root));
    std::fs::write(&cut, &stale).expect("cut");
    hooks(root);

    let home = super::super::command::support::guard(&[], plumb::version!("PLUMB"));
    let output = isolated(root, home.path());
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = String::from_utf8_lossy(&output.stdout);
    assert!(report.contains("recorded"), "{report}");
    let standing = show(bare.path(), "--format=%H --no-patch release/v1.2.0")
        .trim()
        .to_string();
    let parent = show(bare.path(), &format!("--format=%P --no-patch {standing}"));
    assert_eq!(
        parent.trim(),
        stale,
        "datum and refreshed proof must share one commit"
    );
    let message = show(bare.path(), &format!("--format=%B --no-patch {standing}"));
    assert!(plumb::datum::carried(&message).unwrap().is_some());
    plumb::guard::commit(root, &standing).expect("recorded datum proof");
}

fn isolated(root: &Path, home: &Path) -> std::process::Output {
    let binary = Path::new(env!("CARGO_BIN_EXE_plumb"));
    let path = format!(
        "{}:{}",
        binary.parent().expect("Plumb binary parent").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    super::super::command::plumb(root, &["version", "prepare", "--version", "1.2.0"])
        .env("HARNESS_RUN_TIMEOUT_MS", "1000")
        .env("PATH", path)
        .env("PLUMB_HOME", home)
        .env_remove("PLUMB_GUARD_CONFIGURATION")
        .output()
        .expect("plumb")
}

fn hooks(root: &Path) {
    for name in ["pre-commit", "commit-msg"] {
        let hook = root.join(".git/hooks").join(name);
        std::fs::write(&hook, "#!/bin/sh\nexit 0\n").expect("guard hook");
        let mut permissions = std::fs::metadata(&hook)
            .expect("hook metadata")
            .permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o755);
        std::fs::set_permissions(&hook, permissions).expect("executable guard hook");
    }
}

fn show(root: &Path, object: &str) -> String {
    let mut args = vec!["show"];
    args.extend(object.split_whitespace());
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("git show");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}
