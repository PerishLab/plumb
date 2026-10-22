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
        let mut command = Command::new(env!("CARGO_BIN_EXE_plumb"));
        command
            .current_dir(self.root.path())
            .env("PLUMB_HOME", self.home.path())
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
