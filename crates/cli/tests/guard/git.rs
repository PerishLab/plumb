use std::path::Path;
use std::process::Command;

fn git(root: &Path, args: &[&str]) -> String {
    let output = plumb::config::detached("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

#[test]
fn context() {
    let root = tempfile::tempdir().unwrap();
    let foreign = tempfile::tempdir().unwrap();
    let home = super::support::home();
    git(root.path(), &["init", "-q", "-b", "feature"]);
    git(root.path(), &["config", "user.name", "Plumb Test"]);
    git(
        root.path(),
        &["config", "user.email", "plumb@example.invalid"],
    );
    let name = "changed space 文";
    std::fs::write(root.path().join(name), "before\n").unwrap();
    git(root.path(), &["add", "."]);
    git(root.path(), &["commit", "-q", "-m", "base"]);
    let base = git(root.path(), &["rev-parse", "HEAD"]);
    std::fs::write(root.path().join(name), "after\n").unwrap();
    git(root.path(), &["add", "."]);
    git(root.path(), &["commit", "-q", "-m", "head"]);
    let head = git(root.path(), &["rev-parse", "HEAD"]);
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .arg("guard")
        .arg(root.path())
        .args(["--base", &base, "--head", &head, "--write", ".", "--json"])
        .env("PLUMB_HOME", home.path())
        .env("GIT_DIR", foreign.path().join("absent"))
        .env("GIT_WORK_TREE", foreign.path())
        .env("GIT_INDEX_FILE", foreign.path().join("absent-index"))
        .env("GIT_NAMESPACE", "foreign")
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "core.bare")
        .env("GIT_CONFIG_VALUE_0", "true")
        .current_dir(root.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["base"], base);
    assert_eq!(report["head"], head);
    assert_eq!(report["changed"], serde_json::json!([name]));
    assert_eq!(report["ok"], true);
    assert_eq!(std::fs::read_dir(foreign.path()).unwrap().count(), 0);
}
