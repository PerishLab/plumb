use serde_json::Value;
use std::process::{Command, Output};

struct Repo {
    fixture: tempfile::TempDir,
    base: String,
    head: String,
}

impl Repo {
    fn new() -> Self {
        let fixture = tempfile::tempdir().expect("fixture");
        let root = fixture.path();
        Self::git(root, &["init", "-q"]);
        Self::git(root, &["config", "user.name", "Plumb Test"]);
        Self::git(root, &["config", "user.email", "plumb@example.invalid"]);
        std::fs::create_dir(root.join("src")).expect("src");
        std::fs::write(root.join("src/lib.rs"), "one\n").expect("base");
        let base = Self::commit(root, "base");
        std::fs::write(root.join("src/lib.rs"), "two\n").expect("head");
        std::fs::write(root.join("README.md"), "outside\n").expect("outside");
        let head = Self::commit(root, "head");
        Self {
            fixture,
            base,
            head,
        }
    }

    fn git(root: &std::path::Path, args: &[&str]) -> Output {
        let output = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .expect("git");
        assert!(output.status.success());
        output
    }

    fn commit(root: &std::path::Path, message: &str) -> String {
        Self::git(root, &["add", "-A"]);
        Self::git(root, &["commit", "-q", "-m", message]);
        String::from_utf8(Self::git(root, &["rev-parse", "HEAD"]).stdout)
            .expect("utf8")
            .trim()
            .to_owned()
    }

    fn plumb(&self, write: &str) -> Output {
        Command::new(env!("CARGO_BIN_EXE_plumb"))
            .args(["precommit", "--json"])
            .arg(self.fixture.path())
            .args(["--base", &self.base, "--head", &self.head, "--write", write])
            .output()
            .expect("plumb")
    }
}

#[test]
fn outside() {
    let repo = Repo::new();
    let output = repo.plumb("src");
    assert!(!output.status.success());
    assert!(output.stderr.is_empty());
    let report: Value = serde_json::from_slice(&output.stdout).expect("report");
    assert_eq!(report["schema"], "plumb.precommit/v1");
    assert_eq!(report["base"], repo.base);
    assert_eq!(report["head"], repo.head);
    assert_eq!(
        report["changed"],
        serde_json::json!(["README.md", "src/lib.rs"])
    );
    assert_eq!(report["outside"], serde_json::json!(["README.md"]));
    assert_eq!(report["ok"], false);
}

#[test]
fn accepts() {
    let repo = Repo::new();
    let output = repo.plumb(".");
    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).expect("report");
    assert_eq!(report["outside"], serde_json::json!([]));
    assert_eq!(report["ok"], true);
}
