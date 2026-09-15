use super::super::command::support;
use std::path::Path;
use std::process::Command;

#[path = "retry.rs"]
mod retry;

#[path = "evidence.rs"]
mod evidence;

struct Fixture {
    root: tempfile::TempDir,
    bare: tempfile::TempDir,
    home: tempfile::TempDir,
    base: String,
    source: String,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let bare = tempfile::tempdir().unwrap();
        let home = support::depot(&[]);
        git(bare.path(), &["init", "--bare", "-q"]);
        git(root.path(), &["init", "-q"]);
        git(root.path(), &["config", "user.name", "Fixture"]);
        git(
            root.path(),
            &["config", "user.email", "fixture@example.test"],
        );
        git(
            root.path(),
            &["remote", "add", "origin", bare.path().to_str().unwrap()],
        );
        std::fs::create_dir(root.path().join("src")).unwrap();
        std::fs::write(root.path().join(".gitignore"), "target/\n").unwrap();
        std::fs::write(
            root.path().join("Cargo.lock"),
            "version = 4\n\n[[package]]\nname = \"pick-fixture\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        std::fs::write(root.path().join("plumb.toml"), "[workflow.hash.guard]\ntest=['Cargo.toml','src']\n[release]\nproduct='probe'\nauthority='https://releases.test'\nbinaries=['probe']\ntargets=['x86_64-unknown-linux-gnu']\n").unwrap();
        std::fs::write(
            root.path().join("Cargo.toml"),
            "[package]\nname = \"pick-fixture\"\nversion = \"1.0.0\"\nedition = \"2024\"\n\n[workspace.package]\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        std::fs::write(
            root.path().join("src/lib.rs"),
            "pub fn value() -> u8 {\n    0\n}\n",
        )
        .unwrap();
        git(root.path(), &["add", "."]);
        git(root.path(), &["commit", "-qm", "base"]);
        git(root.path(), &["branch", "-M", "release/v1.0.0"]);
        git(root.path(), &["push", "-qu", "origin", "release/v1.0.0"]);
        let base = git(root.path(), &["rev-parse", "HEAD"]);
        git(root.path(), &["switch", "-qc", "source"]);
        std::fs::write(
            root.path().join("src/lib.rs"),
            "pub fn value() -> u8 {\n    1\n}\n",
        )
        .unwrap();
        git(root.path(), &["commit", "-qam", "source"]);
        let source = git(root.path(), &["rev-parse", "HEAD"]);
        git(root.path(), &["switch", "-q", "release/v1.0.0"]);
        Self {
            root,
            bare,
            home,
            base,
            source,
        }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_plumb"));
        command
            .current_dir(self.root.path())
            .env("PLUMB_HOME", self.home.path())
            .env_remove("RUSTFLAGS")
            .args([
                "version",
                "pick",
                "--version",
                "v1.0.0",
                "--commit",
                &self.source,
            ]);
        command
    }

    fn picked(&self) -> String {
        git(self.root.path(), &["cherry-pick", "-x", &self.source]);
        git(self.root.path(), &["rev-parse", "HEAD"])
    }

    fn remote(&self) -> String {
        git(self.bare.path(), &["rev-parse", "release/v1.0.0"])
    }

    fn refused(&self, expected: &str) {
        let head = git(self.root.path(), &["rev-parse", "HEAD"]);
        let remote = self.remote();
        let output = self.command().output().unwrap();
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(git(self.root.path(), &["rev-parse", "HEAD"]), head);
        assert_eq!(self.remote(), remote);
    }

    fn accepted(&self) {
        let output = self.command().output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let head = git(self.root.path(), &["rev-parse", "HEAD"]);
        assert_eq!(self.remote(), head);
        let proof = plumb::guard::commit(self.root.path(), "HEAD").unwrap();
        assert_eq!(
            proof.tree,
            git(self.root.path(), &["rev-parse", "HEAD^{tree}"])
        );
        assert_eq!(git(self.root.path(), &["rev-parse", "HEAD^"]), self.base);
    }
}

#[test]
fn preflight() {
    let fixture = Fixture::new();
    let missing = tempfile::tempdir().unwrap();
    let output = fixture
        .command()
        .env("PLUMB_HOME", missing.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("pick configuration preflight"), "{error}");
    assert!(!error.contains("panicked"), "{error}");
    assert_eq!(
        git(fixture.root.path(), &["rev-parse", "HEAD"]),
        fixture.base
    );
    assert_eq!(fixture.remote(), fixture.base);
}

#[test]
fn resumes() {
    let fixture = Fixture::new();
    fixture.picked();
    fixture.accepted();
    let before = fixture.remote();
    let repeated = fixture.command().output().unwrap();
    assert!(
        repeated.status.success(),
        "{}",
        String::from_utf8_lossy(&repeated.stderr)
    );
    assert!(String::from_utf8_lossy(&repeated.stdout).contains("nothing moved"));
    assert!(!String::from_utf8_lossy(&repeated.stderr).contains("guard guard/"));
    assert_eq!(fixture.remote(), before);
}

#[test]
fn guard() {
    let fixture = Fixture::new();
    let output = fixture
        .command()
        .env("RUSTFLAGS", "--plumb-invalid-compiler-argument")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let head = git(fixture.root.path(), &["rev-parse", "HEAD"]);
    assert_ne!(head, fixture.base);
    assert_eq!(fixture.remote(), fixture.base);
    assert!(git(fixture.root.path(), &["status", "--short"]).is_empty());
    fixture.accepted();
}

#[test]
fn forged() {
    let fixture = Fixture::new();
    fixture.picked();
    std::fs::write(
        fixture.root.path().join("src/lib.rs"),
        "pub fn value() -> u8 {\n    9\n}\n",
    )
    .unwrap();
    git(
        fixture.root.path(),
        &["commit", "-qam", "forged", "--amend"],
    );
    fixture.refused("does not name requested source");
    git(
        fixture.root.path(),
        &[
            "commit",
            "--amend",
            "-qm",
            &format!("forged\n\n(cherry picked from commit {})", fixture.source),
        ],
    );
    fixture.refused("differs from the replayed source tree");
}

#[test]
fn moved() {
    let fixture = Fixture::new();
    fixture.picked();
    git(
        fixture.bare.path(),
        &[
            "fetch",
            "-q",
            fixture.root.path().to_str().unwrap(),
            "source",
        ],
    );
    git(
        fixture.bare.path(),
        &["update-ref", "refs/heads/release/v1.0.0", &fixture.source],
    );
    fixture.refused("current remote base");
}

#[test]
fn unfinished() {
    let fixture = Fixture::new();
    fixture.picked();
    std::fs::create_dir(fixture.root.path().join(".git/sequencer")).unwrap();
    fixture.refused("unfinished Git operation");
}

#[test]
fn marked() {
    let fixture = Fixture::new();
    fixture.picked();
    git(
        fixture.root.path(),
        &["tag", "-a", "v1.0.0", "-m", "fixture"],
    );
    fixture.refused("stable marker v1.0.0 already stands");
}

#[test]
fn dirty() {
    let fixture = Fixture::new();
    fixture.picked();
    std::fs::write(fixture.root.path().join("src/lib.rs"), "dirty\n").unwrap();
    fixture.refused("clean worktree");
}

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}
