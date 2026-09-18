use super::stable::{repo, run};
use std::path::Path;
use std::process::Command;

pub fn lined(root: &Path, origin: &str, bare: &Path, line: &str) -> String {
    run(Command::new("git").args(["init", "-q", "--bare"]).arg(bare));
    let local = format!("file://{}", bare.display());
    repo(root, &local);
    let forge = origin
        .strip_suffix("/test/probe.git")
        .expect("fixture origin");
    run(Command::new("git")
        .args(["config", "plumb.test-forgejo-url", forge])
        .current_dir(root));
    run(Command::new("git")
        .args(["config", "user.email", "probe@test"])
        .current_dir(root));
    run(Command::new("git")
        .args(["config", "user.name", "probe"])
        .current_dir(root));
    run(Command::new("git").args(["add", "-A"]).current_dir(root));
    run(Command::new("git")
        .args(["commit", "-q", "-m", "Stand the probe up"])
        .current_dir(root));
    run(Command::new("git")
        .args(["push", "-q", "origin", &format!("HEAD:refs/heads/{line}")])
        .current_dir(root));
    run(Command::new("git")
        .args(["push", "-q", "origin", "HEAD:refs/heads/main"])
        .current_dir(root));
    let head = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()
        .expect("git");
    let head = String::from_utf8_lossy(&head.stdout).trim().to_string();
    run(Command::new("git")
        .args(["update-ref", "refs/remotes/origin/main", &head])
        .current_dir(root));
    head
}
