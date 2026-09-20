use super::support;
mod cache;
mod environment;
mod governed;
mod physical;
mod world;
use serde_json::Value;
use std::process::{Command, Output};

pub(super) const WORKFLOW: &str = r#"
[suite]
cargo = ["Cargo.lock", "Cargo.toml", "crates"]

[execution.cargo]
inherit = ["PATH", "HOME", "TMPDIR", "LANG", "TZ", "CARGO_HOME", "RUSTUP_HOME", "RUSTUP_TOOLCHAIN"]
managed = ["CARGO", "CARGO_TARGET_DIR", "CARGO_MANIFEST_*", "CARGO_PKG_*", "CARGO_BIN_EXE_*", "CARGO_CFG_*", "CARGO_PRIMARY_PACKAGE", "CARGO_MAKEFLAGS", "RUST_RECURSION_COUNT", "RUSTUP_TOOLCHAIN_SOURCE"]
reject = ["CARGO_*", "RUST*", "CC", "CC_*", "LD_PRELOAD", "DYLD_*"]
"#;

fn seat() -> tempfile::TempDir {
    support::depot(&[("rules/workflow.toml", WORKFLOW)])
}
pub(super) struct Repo {
    fixture: tempfile::TempDir,
    pub base: String,
    pub head: String,
}
impl Repo {
    pub fn new() -> Self {
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

    pub fn plumb(&self, write: &str) -> Output {
        let depot = seat();
        support::plumb()
            .args(["guard", "--json"])
            .arg(self.fixture.path())
            .args(["--base", &self.base, "--head", &self.head, "--write", write])
            .env("PLUMB_HOME", depot.path())
            .output()
            .expect("plumb")
    }
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

#[test]
fn staged() {
    let fixture = tempfile::tempdir().expect("fixture");
    let home = tempfile::tempdir().expect("home");
    let depot = seat();
    let root = fixture.path();
    Repo::git(root, &["init", "-q"]);
    Repo::git(root, &["config", "user.name", "Plumb Test"]);
    Repo::git(root, &["config", "user.email", "plumb@example.invalid"]);
    Repo::git(
        root,
        &[
            "remote",
            "add",
            "origin",
            "https://git.example.invalid/Example/probe.git",
        ],
    );
    std::fs::create_dir(root.join("src")).expect("src");
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"probe\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("manifest");
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn answer() -> u8 {\n    42\n}\n",
    )
    .expect("source");
    std::fs::write(
        root.join("plumb.toml"),
        "[workflow.hash.guard]\nrust = [\"Cargo.toml\", \"Cargo.lock\", \"src\"]\n",
    )
    .expect("shape");
    Repo::git(root, &["add", "Cargo.toml", "src/lib.rs", "plumb.toml"]);
    let lock = Command::new("cargo")
        .args(["generate-lockfile", "--offline"])
        .current_dir(root)
        .output()
        .expect("lock");
    assert!(
        lock.status.success(),
        "{}",
        String::from_utf8_lossy(&lock.stderr)
    );
    Repo::git(root, &["add", "Cargo.lock"]);

    let run = || {
        support::plumb()
            .args(["guard", ".", "--json"])
            .current_dir(root)
            .env("PLUMB_HOME", depot.path())
            .env("PLUMB_GUARD_CONFIGURATION", home.path())
            .env("GIT_DIR", root.join(".git"))
            .env("GIT_INDEX_FILE", ".git/index")
            .env("GIT_WORK_TREE", root)
            .output()
            .expect("precommit")
    };
    let first = run();
    assert!(
        first.status.success(),
        "{}{}",
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(String::from_utf8_lossy(&first.stderr).contains("guard guard/rust"));
    let message = root.join("message");
    std::fs::write(&message, "candidate\n").expect("message");
    let attached = support::plumb()
        .args(["guard", ".", "--attach"])
        .arg(&message)
        .current_dir(root)
        .env("PLUMB_HOME", depot.path())
        .output()
        .expect("attach");
    assert!(
        attached.status.success(),
        "{}",
        String::from_utf8_lossy(&attached.stderr)
    );
    Repo::git(
        root,
        &[
            "commit",
            "-q",
            "--no-verify",
            "-F",
            message.to_str().expect("message"),
        ],
    );
    plumb::guard::commit(root, "HEAD").expect("committed proof");

    std::fs::write(root.join("NOTES"), "unrelated\n").expect("unrelated");
    Repo::git(root, &["add", "NOTES"]);
    let second = run();
    assert!(
        second.status.success(),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );
    assert!(
        !String::from_utf8_lossy(&second.stderr).contains("guard guard/rust"),
        "unchanged action must not start: {}",
        String::from_utf8_lossy(&second.stderr)
    );
}
