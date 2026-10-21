use super::{Context, Session, record::Hint};
use crate::command::guard::precommit::cargo::Lease;
use std::path::Path;
use std::process::{Command, Output};

fn isolated(name: &str) -> bool {
    if std::env::var_os("PLUMB_MATERIAL_TEST").is_some() {
        return false;
    }
    let home = tempfile::tempdir().unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            &format!("command::cache::transport::tests::{name}"),
            "--nocapture",
        ])
        .env("PLUMB_MATERIAL_TEST", "child")
        .env("PLUMB_HOME", home.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    true
}

struct Repository(tempfile::TempDir);

impl Repository {
    fn new() -> Self {
        let root = Self(tempfile::tempdir().unwrap());
        root.seed();
        root
    }

    fn root(&self) -> &Path {
        self.0.path()
    }

    fn seed(&self) {
        self.git(&["init", "-q", "-b", "topic"]);
        self.write(
            "Cargo.toml",
            "[package]\nname='probe'\nversion='0.1.0'\nedition='2024'\n",
        );
        self.write("src/lib.rs", "pub fn original() -> u8 { 1 }\n");
        self.write("src/main.rs", "fn main() { probe::original(); }\n");
        let output = Command::new("cargo")
            .args(["generate-lockfile", "--offline"])
            .current_dir(self.root())
            .output()
            .unwrap();
        assert!(output.status.success());
        self.git(&["add", "Cargo.toml", "Cargo.lock", "src"]);
    }

    fn git(&self, args: &[&str]) {
        let output = plumb::config::detached("git")
            .arg("-C")
            .arg(self.root())
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn write(&self, path: &str, text: &str) {
        let path = self.root().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn check(&self, lease: &Lease) -> Output {
        let mut command = Command::new("cargo");
        command
            .args(["check", "--locked", "--offline"])
            .current_dir(self.root())
            .env("CARGO_TARGET_DIR", &lease.root)
            .env("CARGO_INCREMENTAL", "0");
        for name in ["RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER"] {
            command.env_remove(name);
        }
        command.output().unwrap()
    }

    fn pass(&self, lease: &Lease) -> Output {
        let output = self.check(lease);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }
}

#[test]
fn fresh() {
    if isolated("fresh") {
        return;
    }
    let repo = Repository::new();
    let context = Context {
        image: format!("sha256:{}", "a".repeat(64)),
    };
    let lease = Lease::new(repo.root()).unwrap();
    let session = Session::begin(repo.root(), &lease, "same-tools-and-options", &context)
        .unwrap()
        .unwrap();
    repo.pass(&lease);
    session.seal().unwrap();
    let stamp = std::fs::metadata(repo.root().join("src/lib.rs"))
        .unwrap()
        .modified()
        .unwrap();
    std::fs::remove_dir_all(repo.root()).unwrap();
    std::fs::create_dir(repo.root()).unwrap();
    repo.seed();
    let session = Session::begin(repo.root(), &lease, "same-tools-and-options", &context)
        .unwrap()
        .unwrap();
    let output = repo.pass(&lease);
    assert!(
        !String::from_utf8_lossy(&output.stderr).contains("Checking probe"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::metadata(repo.root().join("src/lib.rs"))
            .unwrap()
            .modified()
            .unwrap(),
        stamp
    );
    session.seal().unwrap();
}

#[test]
fn failure() {
    if isolated("failure") {
        return;
    }
    let repo = Repository::new();
    let context = Context {
        image: format!("sha256:{}", "b".repeat(64)),
    };
    let lease = Lease::new(repo.root()).unwrap();
    let session = Session::begin(repo.root(), &lease, "world", &context)
        .unwrap()
        .unwrap();
    repo.pass(&lease);
    session.seal().unwrap();
    let stamp = std::fs::metadata(repo.root().join("src/main.rs"))
        .unwrap()
        .modified()
        .unwrap();
    repo.write("src/lib.rs", "pub fn changed() -> u8 { 2 }\n");
    repo.write("src/main.rs", "fn main() { probe::changed(); }\n");
    let _failed = Session::begin(repo.root(), &lease, "world", &context)
        .unwrap()
        .unwrap();
    repo.pass(&lease);
    assert!(
        !Hint::read(&lease.root).unwrap().unwrap().ready,
        "an unfinished attempt cannot reseal material"
    );
    repo.write("src/main.rs", "fn main() { probe::original(); }\n");
    std::fs::File::options()
        .read(true)
        .write(true)
        .open(repo.root().join("src/main.rs"))
        .unwrap()
        .set_modified(stamp)
        .unwrap();
    let _next = Session::begin(repo.root(), &lease, "world", &context)
        .unwrap()
        .unwrap();
    let output = repo.check(&lease);
    assert!(
        !output.status.success(),
        "stale B metadata cannot validate invalid mixed source"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot find function `original`"));
}

#[test]
fn damaged() {
    if isolated("damaged") {
        return;
    }
    let repo = Repository::new();
    let context = Context {
        image: format!("sha256:{}", "c".repeat(64)),
    };
    let lease = Lease::new(repo.root()).unwrap();
    let session = Session::begin(repo.root(), &lease, "world", &context)
        .unwrap()
        .unwrap();
    repo.pass(&lease);
    session.seal().unwrap();
    let hint = lease.root.join("source.json");
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&hint).unwrap()).unwrap();
    value["stamp"]["seconds"] = serde_json::json!(u64::MAX);
    std::fs::write(&hint, serde_json::to_vec(&value).unwrap()).unwrap();
    let _next = Session::begin(repo.root(), &lease, "world", &context)
        .unwrap()
        .unwrap();
    let output = repo.pass(&lease);
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Checking probe"),
        "future metadata must force recompilation"
    );
}

#[test]
fn worlds() {
    if isolated("worlds") {
        return;
    }
    let repo = Repository::new();
    let context = Context {
        image: format!("sha256:{}", "d".repeat(64)),
    };
    let lease = Lease::new(repo.root()).unwrap();
    let first = Session::begin(repo.root(), &lease, "original-options", &context)
        .unwrap()
        .unwrap();
    repo.pass(&lease);
    first.seal().unwrap();
    let next = Session::begin(repo.root(), &lease, "changed-options-or-tools", &context)
        .unwrap()
        .unwrap();
    let output = repo.pass(&lease);
    assert!(String::from_utf8_lossy(&output.stderr).contains("Checking probe"));
    next.seal().unwrap();
    let changed = Context {
        image: format!("sha256:{}", "e".repeat(64)),
    };
    let _next = Session::begin(repo.root(), &lease, "changed-options-or-tools", &changed)
        .unwrap()
        .unwrap();
    let output = repo.pass(&lease);
    assert!(String::from_utf8_lossy(&output.stderr).contains("Checking probe"));
}

#[test]
fn features() {
    if isolated("features") {
        return;
    }
    let repo = Repository::new();
    let manifest = "[package]\nname='probe'\nversion='0.1.0'\nedition='2024'\n[features]\nenabled=[]\ndefault=['enabled']\n";
    repo.write("Cargo.toml", manifest);
    repo.write(
        "src/lib.rs",
        "#[cfg(feature = \"enabled\")]\npub fn original() -> u8 { 1 }\n",
    );
    let context = Context {
        image: format!("sha256:{}", "f".repeat(64)),
    };
    let lease = Lease::new(repo.root()).unwrap();
    let first = Session::begin(repo.root(), &lease, "world", &context)
        .unwrap()
        .unwrap();
    repo.pass(&lease);
    first.seal().unwrap();
    repo.write(
        "Cargo.toml",
        &manifest.replace("default=['enabled']", "default=[]"),
    );
    let _next = Session::begin(repo.root(), &lease, "world", &context)
        .unwrap()
        .unwrap();
    let output = repo.check(&lease);
    assert!(
        !output.status.success(),
        "a changed feature set must not reuse a stale success"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot find function `original`"));
}
