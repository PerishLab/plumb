use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

struct Fixture {
    root: tempfile::TempDir,
    home: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        Self {
            root: tempfile::tempdir().unwrap(),
            home: super::configuration::support::home(),
        }
    }

    fn run(&self, args: &[&str], report: Option<&Path>) -> Output {
        let mut command = super::configuration::support::plumb();
        command
            .current_dir(self.root.path())
            .env("PLUMB_HOME", self.home.path())
            .env_remove("PLUMB_GUARD_STRENGTH")
            .env_remove("PLUMB_GUARD_BOUNDARY")
            .env_remove("PLUMB_GUARD_CONTAINER")
            .env("GH_TOKEN", "private-sentinel-token")
            .args(args);
        if let Some(report) = report {
            command.arg("--diagnostic").arg(report);
        }
        command.output().unwrap()
    }

    fn records(&self, path: &Path) -> Vec<Value> {
        let text = std::fs::read_to_string(path).unwrap();
        assert!(!text.contains("private-sentinel-token"));
        text.lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }
}

#[test]
fn refused() {
    let fixture = Fixture::new();
    let report = fixture.home.path().join("diagnostic.jsonl");
    let baseline = fixture.run(&["guard", "--json"], None);
    let observed = fixture.run(&["guard", "--json"], Some(&report));
    assert_eq!(observed.status.code(), baseline.status.code());
    assert_eq!(observed.stdout, baseline.stdout);
    assert_eq!(observed.stderr, baseline.stderr);
    assert!(!observed.status.success());
    let records = fixture.records(&report);
    assert_eq!(records[0]["kind"], "start");
    assert_eq!(
        records.last().unwrap()["data"]["exit_code"],
        observed.status.code().unwrap()
    );
    assert!(
        records
            .iter()
            .all(|record| record["schema"] == "plumb.guard-diagnostic/v1")
    );
    let bytes = std::fs::read(&report).unwrap();
    let repeated = fixture.run(&["guard", "--json"], Some(&report));
    assert_eq!(repeated.status.code(), baseline.status.code());
    assert_eq!(repeated.stdout, baseline.stdout);
    assert_eq!(std::fs::read(&report).unwrap(), bytes);
    assert!(
        String::from_utf8_lossy(&repeated.stderr).contains("cannot create new diagnostic file")
    );
}

#[test]
fn transport() {
    let fixture = Fixture::new();
    assert!(
        Command::new("git")
            .args(["init", "-q", "-b", "main"])
            .current_dir(fixture.root.path())
            .status()
            .unwrap()
            .success()
    );
    let report = fixture.home.path().join("diagnostic.jsonl");
    let baseline = fixture.run(&["cache", "transport", "--json"], None);
    let observed = fixture.run(&["cache", "transport", "--json"], Some(&report));
    assert!(
        observed.status.success(),
        "{}",
        String::from_utf8_lossy(&observed.stderr)
    );
    assert_eq!(observed.stdout, baseline.stdout);
    assert_eq!(observed.stderr, baseline.stderr);
    let records = fixture.records(&report);
    assert!(
        records.iter().any(|record| record["kind"] == "enter"
            && record["data"]["operation"] == "material.transport")
    );
    assert!(
        records
            .iter()
            .any(|record| record["kind"] == "material.receipt" && record["data"]["cargo"] == false)
    );
    assert_eq!(records.last().unwrap()["data"]["exit_code"], 0);
}

#[test]
fn unwritable() {
    let fixture = Fixture::new();
    assert!(
        Command::new("git")
            .args(["init", "-q", "-b", "main"])
            .current_dir(fixture.root.path())
            .status()
            .unwrap()
            .success()
    );
    let held = fixture.home.path().join("held");
    std::fs::write(&held, "preserved").unwrap();
    let baseline = fixture.run(&["cache", "transport", "--json"], None);
    let result = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .current_dir(fixture.root.path())
        .env("PLUMB_HOME", fixture.home.path())
        .args(["cache", "transport", "--json", "--diagnostic"])
        .arg(&held)
        .stderr(std::process::Stdio::from(
            std::fs::File::open(&held).unwrap(),
        ))
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(0));
    assert_eq!(result.stdout, baseline.stdout);
    assert_eq!(std::fs::read_to_string(held).unwrap(), "preserved");
}

#[test]
fn proved() {
    let fixture = Fixture::new();
    let root = fixture.root.path();
    std::fs::create_dir(root.join("src")).unwrap();
    for (path, text) in [
        (
            "Cargo.toml",
            "[package]\nname=\"probe\"\nversion=\"0.1.0\"\nedition=\"2024\"\n",
        ),
        (
            "src/lib.rs",
            "#[test]\nfn answer() {\n    assert_eq!(2 + 2, 4);\n}\n",
        ),
        (
            "plumb.toml",
            "[identity]\nns=\"probe\"\n[workflow.hash.guard]\ntest=[\"Cargo.toml\",\"Cargo.lock\",\"src\"]\n",
        ),
    ] {
        std::fs::write(root.join(path), text).unwrap();
    }
    let git = |args: &[&str]| {
        let result = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        result.stdout
    };
    git(&["init", "-q", "-b", "feature/probe"]);
    git(&[
        "remote",
        "add",
        "origin",
        "https://github.com/Example/probe.git",
    ]);
    let lock = super::configuration::support::outside("cargo")
        .args(["generate-lockfile", "--offline"])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(lock.status.success());
    git(&["add", "."]);
    let tree = String::from_utf8(git(&["write-tree"])).unwrap();
    let report = fixture.home.path().join("guard.jsonl");
    let result = fixture.run(&["guard", "--json"], Some(&report));
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let proof: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(proof["tree"], tree.trim());
    let records = fixture.records(&report);
    assert!(records.iter().any(|record| record["kind"] == "guard.proof"
        && record["data"]["proof"]["repository"] == "Example/probe"));
    assert!(
        records
            .iter()
            .any(|record| record["kind"] == "command.exit" && record["data"]["code"] == 0)
    );
    assert_eq!(records.last().unwrap()["data"]["exit_code"], 0);
    assert_eq!(records.last().unwrap()["data"]["capture"], "complete");
}
