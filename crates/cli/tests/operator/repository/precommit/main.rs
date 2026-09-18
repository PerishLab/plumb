use super::support;
mod cache;
mod environment;
mod physical;
mod world;
use serde_json::Value;
use std::process::{Command, Output};

const WORKFLOW: &str = r#"
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
#[cfg(unix)]
fn governed() {
    let fixture = tempfile::tempdir().expect("fixture");
    let seat = fixture.path().join("probe");
    std::fs::create_dir(&seat).expect("repository");
    let root = seat.as_path();
    Repo::git(root, &["init", "-q"]);
    Repo::git(root, &["config", "user.name", "Plumb Test"]);
    Repo::git(root, &["config", "user.email", "plumb@example.invalid"]);
    Repo::git(
        root,
        &[
            "remote",
            "add",
            "origin",
            "ssh://git@git.perish.top/PerishFire/probe.git",
        ],
    );
    std::fs::create_dir(root.join("src")).expect("source");
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"probe\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("manifest");
    std::fs::write(root.join(".gitignore"), "target/\n").expect("ignore");
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn answer() -> u8 {\n    42\n}\n",
    )
    .expect("source");
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
    Repo::git(
        root,
        &[
            "add",
            ".gitignore",
            "Cargo.toml",
            "Cargo.lock",
            "src/lib.rs",
        ],
    );
    let profile = super::world::profile(
        "probe",
        "[layout]\n[[layout.seat]]\npath = \"src\"\n[[layout.file]]\nname = [\".gitignore\", \"Cargo.toml\", \"Cargo.lock\"]\n",
        "[comment]\nallow = false\n[limit]\nblock = 4\nfanout = 10\nfile = 300\nmarkup = 8\nparam = 4\npath = 3\n[word]\nsingle = true\n",
    );
    let digest = plumb::depot::sha(profile.as_bytes());
    let catalog = format!(
        "schema = \"plumb.products/v2\"\n\n[[product]]\nidentity = \"git.perish.top/PerishFire/probe\"\nprofile = \"{digest}\"\n"
    );
    let path = format!("profiles/{digest}.toml");
    let home = support::home(&[
        ("rules/products.toml", &catalog),
        (&path, &profile),
        ("rules/workflow.toml", WORKFLOW),
    ]);
    super::world::hooks(root);
    let binary = std::path::Path::new(env!("CARGO_BIN_EXE_plumb"));
    let path = format!(
        "{}:{}",
        binary.parent().expect("Plumb binary directory").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let run = || {
        support::plumb()
            .args(["guard", "."])
            .current_dir(root)
            .env_remove("PLUMB_HOME")
            .env_remove("PLUMB_GUARD_CONFIGURATION")
            .env_remove("PLUMB_GUARD_DEPOT")
            .env_remove("PLUMB_GUARD_VIEW")
            .env("HOME", home.path())
            .env("PATH", &path)
            .output()
            .expect("guard")
    };
    let output = run();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let held = String::from_utf8_lossy(&output.stderr);
    assert!(held.contains("guard guard/plumb"), "{held}");
    assert!(held.contains("guard guard/ectropy"), "{held}");
    assert!(!held.contains("must not carry plumb.toml or ectropy.toml"));
    assert!(!root.join("plumb.toml").exists());
    assert!(!root.join("ectropy.toml").exists());
    std::fs::write(root.join("ectropy.toml"), "").expect("second expression");
    Repo::git(root, &["add", "ectropy.toml"]);
    let refusal = run();
    assert!(!refusal.status.success());
    assert!(
        String::from_utf8_lossy(&refusal.stderr)
            .contains("must not carry plumb.toml or ectropy.toml")
    );
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
