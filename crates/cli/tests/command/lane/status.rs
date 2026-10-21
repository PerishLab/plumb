use super::{fixture, git, run};
use serde_json::Value;

#[test]
fn local() {
    let root = fixture();
    let path = root.path().to_str().unwrap();
    let index = root.path().join(".git/index");
    let before = std::fs::read(&index).unwrap();
    let package = root.path().join("apps/review/package.json");
    std::fs::write(&package, std::fs::read(&package).unwrap()).unwrap();
    let output = run(&[
        "lane",
        "status",
        path,
        "--app",
        "crest",
        "--lane",
        "design",
        "--require",
        "deploy",
        "--json",
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(std::fs::read(index).unwrap(), before);
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["schema"], "plumb.lane.status/v1");
    assert_eq!(report["declaration"]["target"]["lane"], "design");
    assert_eq!(report["declaration"]["target"]["app"], "crest");
    assert_eq!(
        report["source"]["commit"],
        git(root.path(), &["rev-parse", "HEAD"])
    );
    assert_eq!(
        report["source"]["tree"],
        git(root.path(), &["rev-parse", "HEAD^{tree}"])
    );
    assert_eq!(report["current"]["outcome"], "unknown");
    assert!(report["artifact"].is_null());
    assert!(
        !String::from_utf8(output.stdout)
            .unwrap()
            .contains("private-provider-value")
    );
    assert!(!root.path().join("SHOULD_NOT_EXIST").exists());
    assert!(!root.path().join("apps/review/SHOULD_NOT_EXIST").exists());
    assert_eq!(git(root.path(), &["status", "--porcelain"]), "");
}

#[test]
fn selection() {
    let root = fixture();
    let path = root.path().to_str().unwrap();
    for lane in ["preview", "preview.design", "v1.2.3", "absent"] {
        let output = run(&[
            "lane", "status", path, "--app", "crest", "--lane", lane, "--json",
        ]);
        assert!(!output.status.success(), "{lane}");
        assert!(output.stdout.is_empty());
    }
    let output = run(&[
        "lane",
        "status",
        path,
        "--app",
        "crest",
        "--lane",
        "review",
        "--require",
        "publish",
        "--json",
    ]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("does not declare Publish"));
    let output = run(&["lane", "status", path, "--app", "crest", "--lane", "review"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("review"));
    assert!(text.contains("current unknown"));
    assert!(!text.contains("private-provider-value"));
}

#[test]
fn dirty() {
    let root = fixture();
    let path = root.path().to_str().unwrap();
    let head = git(root.path(), &["rev-parse", "HEAD"]);
    let marker = root.path().join("preserve.txt");
    std::fs::write(&marker, "preserve").unwrap();
    let output = run(&[
        "lane", "status", path, "--app", "crest", "--lane", "design", "--json",
    ]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("clean exact source"));
    assert_eq!(std::fs::read_to_string(marker).unwrap(), "preserve");
    assert_eq!(git(root.path(), &["rev-parse", "HEAD"]), head);
}

#[test]
#[cfg(unix)]
fn symlink() {
    let root = fixture();
    let secret = tempfile::tempdir().unwrap();
    let target = secret.path().join("private.toml");
    std::fs::write(&target, "private-provider-value").unwrap();
    let manifest = root.path().join("plumb.toml");
    std::fs::remove_file(&manifest).unwrap();
    std::os::unix::fs::symlink(target, manifest).unwrap();
    git(root.path(), &["add", "."]);
    git(root.path(), &["commit", "--quiet", "-m", "symlink"]);
    let output = run(&[
        "lane",
        "status",
        root.path().to_str().unwrap(),
        "--app",
        "crest",
        "--lane",
        "design",
        "--json",
    ]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("private-provider-value"));
}
