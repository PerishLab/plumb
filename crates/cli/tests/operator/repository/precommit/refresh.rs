use super::{Repo, cache, support};
use serde_json::Value;
use std::path::Path;
use std::process::Output;

#[test]
fn rebased() {
    let fixture = cache::fixture();
    let root = fixture.path();
    let home = support::home();
    Repo::git(root, &["commit", "-q", "-m", "base"]);
    Repo::git(root, &["branch", "main"]);
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn answer() -> u8 {\n    43\n}\n",
    )
    .expect("topic");
    Repo::git(root, &["add", "src/lib.rs"]);
    cache::success(&cache::run(root, home.path()));
    let message = root.join("message");
    std::fs::write(&message, "topic\n\nkeeps context\n").expect("message");
    let attached = support::plumb()
        .args(["guard", ".", "--attach"])
        .arg(&message)
        .current_dir(root)
        .env("PLUMB_HOME", home.path())
        .output()
        .expect("attach proof");
    cache::success(&attached);
    Repo::git(
        root,
        &[
            "commit",
            "-q",
            "--no-verify",
            "-F",
            message.to_str().expect("message path"),
        ],
    );
    std::fs::remove_file(&message).expect("remove message");
    plumb::guard::commit(root, "HEAD").expect("original proof");
    Repo::git(root, &["checkout", "-q", "main"]);
    std::fs::write(root.join("UPSTREAM"), "advanced\n").expect("upstream");
    Repo::git(root, &["add", "UPSTREAM"]);
    Repo::git(root, &["commit", "-q", "-m", "advance base"]);
    Repo::git(root, &["checkout", "-q", "topic"]);
    Repo::git(root, &["rebase", "main"]);
    let source = text(root, &["rev-parse", "HEAD"]);
    let tree = text(root, &["rev-parse", "HEAD^{tree}"]);
    let parents = text(root, &["show", "-s", "--format=%P", "HEAD"]);
    let author = text(root, &["show", "-s", "--format=%an%x00%ae%x00%aI", "HEAD"]);
    let committer = text(root, &["show", "-s", "--format=%cn%x00%ce%x00%cI", "HEAD"]);
    let story = narrative(&text(root, &["show", "-s", "--format=%B", "HEAD"]));
    assert!(plumb::guard::commit(root, "HEAD").is_err());
    assert_eq!(text(root, &["rev-list", "--count", "main..HEAD"]), "1");

    let refreshed = run(root, home.path());
    cache::success(&refreshed);
    let report: Value = serde_json::from_slice(&refreshed.stdout).expect("refresh report");
    let head = text(root, &["rev-parse", "HEAD"]);
    assert_eq!(report["schema"], "plumb.guard-refresh/v1");
    assert_eq!(report["source"], source);
    assert_eq!(report["head"], head);
    assert_eq!(report["tree"], tree);
    assert_ne!(report["source"], report["head"]);
    assert_eq!(text(root, &["rev-list", "--count", "main..HEAD"]), "1");
    assert_eq!(text(root, &["show", "-s", "--format=%s", "HEAD"]), "topic");
    assert_eq!(text(root, &["show", "-s", "--format=%P", "HEAD"]), parents);
    assert_eq!(
        text(root, &["show", "-s", "--format=%an%x00%ae%x00%aI", "HEAD"]),
        author
    );
    assert_eq!(
        text(root, &["show", "-s", "--format=%cn%x00%ce%x00%cI", "HEAD"]),
        committer
    );
    assert_eq!(
        narrative(&text(root, &["show", "-s", "--format=%B", "HEAD"])),
        story
    );
    let proof = plumb::guard::commit(root, "HEAD").expect("refreshed proof");
    assert_eq!(report["guard"]["digest"], proof.digest);

    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn answer() -> u8 {\n    44\n}\n",
    )
    .expect("change");
    Repo::git(root, &["add", "src/lib.rs"]);
    Repo::git(
        root,
        &["commit", "-q", "--no-verify", "-m", "change source"],
    );
    assert!(plumb::guard::commit(root, "HEAD").is_err());
}

#[test]
fn refusals() {
    for state in ["dirty", "detached"] {
        let fixture = cache::fixture();
        let root = fixture.path();
        let home = support::home();
        Repo::git(root, &["commit", "-q", "-m", "base"]);
        match state {
            "dirty" => std::fs::write(root.join("DIRTY"), "changed\n").expect("dirty"),
            "detached" => {
                Repo::git(root, &["checkout", "-q", "--detach"]);
            }
            _ => unreachable!(),
        }
        let before = text(root, &["rev-parse", "HEAD"]);
        let output = run(root, home.path());
        assert!(!output.status.success());
        assert_eq!(text(root, &["rev-parse", "HEAD"]), before);
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains(state), "{state}: {error}");
    }
}

fn run(root: &Path, home: &Path) -> Output {
    support::plumb()
        .args(["guard", ".", "--refresh", "--json"])
        .current_dir(root)
        .env("PLUMB_HOME", home)
        .output()
        .expect("refresh")
}

fn text(root: &Path, arguments: &[&str]) -> String {
    String::from_utf8(Repo::git(root, arguments).stdout)
        .expect("git text")
        .trim()
        .to_string()
}

fn narrative(message: &str) -> String {
    message
        .lines()
        .filter(|line| !line.starts_with("Plumb-Guard-Proof: "))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}
