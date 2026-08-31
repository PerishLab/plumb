use super::stable::{command, run};
use super::world::{Court, serve};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn provenance(root: &Path, cut: PathBuf) {
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
