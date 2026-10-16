use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

const BUFFERED: &str = "[locus]\nenabled = true\nendpoint = 'http://127.0.0.1:0'\n";

struct Home(tempfile::TempDir);

impl Home {
    fn new() -> Self {
        isolate();
        Self(tempfile::tempdir().expect("home"))
    }

    fn path(&self) -> &Path {
        self.0.path()
    }

    fn run(&self, key: &str) -> Output {
        self.invoke(&["metadata", key])
    }

    fn invoke(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_plumb"))
            .env("PLUMB_HOME", self.path())
            .env_remove("LOCUS_API")
            .env_remove(locus::collector::PARENT)
            .args(args)
            .output()
            .expect("command")
    }

    fn configure(&self, text: &str) {
        std::fs::write(self.path().join("plumb.toml"), text).expect("configuration");
    }

    fn faults(&self) -> Vec<Value> {
        self.records()
            .into_iter()
            .filter(|atom| atom["payload"]["event"] == "cli.finish")
            .map(|atom| atom["context"]["plumb.fault"].clone())
            .collect()
    }

    fn records(&self) -> Vec<Value> {
        let mut records = Vec::new();
        for entry in std::fs::read_dir(self.path().join("state/locus")).expect("buffer") {
            let path = entry.expect("entry").path();
            let name = path.file_name().expect("name").to_string_lossy();
            if name == "active.jsonl" || name.starts_with("sealed-") {
                let content = std::fs::read_to_string(path).expect("records");
                records.extend(
                    content
                        .lines()
                        .map(|line| serde_json::from_str::<Value>(line).expect("atom")),
                );
            }
        }
        records
    }
}

fn isolate() {
    unsafe {
        std::env::remove_var("LOCUS_API");
        std::env::remove_var(locus::collector::PARENT);
    }
}

#[test]
fn disabled() {
    let home = Home::new();
    let absent = home.run("node.version");
    home.configure("[locus]\nenabled = false\n");
    let disabled = home.run("node.version");
    assert_eq!(absent.status.code(), disabled.status.code());
    assert_eq!(absent.stdout, disabled.stdout);
    assert_eq!(absent.stderr, disabled.stderr);
    assert!(!home.path().join("state/locus").exists());
}

#[test]
fn buffered() {
    let home = Home::new();
    home.configure(BUFFERED);
    for (key, code) in [("node.version", 0), ("unknown", 2)] {
        let result = home.run(key);
        assert_eq!(result.status.code(), Some(code));
    }
    let atoms = home.records();
    assert_eq!(atoms.len(), 10);
    assert!(atoms.iter().all(|atom| atom["producer"] == "plumb"));
    assert!(
        atoms
            .iter()
            .all(|atom| atom["context"]["plumb.command"] == "metadata")
    );
    for code in [0, 2] {
        assert!(atoms.iter().any(
            |atom| atom["payload"]["event"] == "cli.finish" && atom["payload"]["code"] == code
        ));
    }
    for edge in ["enter", "return"] {
        assert_eq!(
            atoms
                .iter()
                .filter(|atom| atom["source"]["edge"] == edge)
                .count(),
            2
        );
    }
}

#[test]
fn isolated() {
    let home = Home::new();
    let baseline = home.run("node.version");
    for config in [
        "[locus]\nenabled = true\nendpoint = 'http://127.0.0.1:0'\n",
        "[locus]\nenabled = 'private-sentinel'\n",
        "[locus]\nenabled = true\nendpoint = ''\n",
    ] {
        home.configure(config);
        let result = home.run("node.version");
        assert_eq!(result.status.code(), baseline.status.code());
        assert_eq!(result.stdout, baseline.stdout);
        assert!(!result.stderr.is_empty(), "diagnostic remains visible");
        let diagnostic = String::from_utf8_lossy(&result.stderr);
        assert!(!diagnostic.contains("private-sentinel"));
        if config.contains("private-sentinel") {
            assert!(diagnostic.contains(&home.path().join("plumb.toml").display().to_string()));
            assert!(diagnostic.contains("check [locus] settings"));
        }
    }
}

#[test]
fn succeeded() {
    let home = Home::new();
    home.configure(BUFFERED);
    assert_eq!(home.run("node.version").status.code(), Some(0));
    assert_eq!(home.faults(), vec![Value::Null]);
}

#[test]
fn refused() {
    let home = Home::new();
    let root = tempfile::tempdir().expect("root");
    let status = Command::new("git")
        .args(["init", "-q", "-b", "main"])
        .current_dir(root.path())
        .status()
        .expect("git");
    assert!(status.success());
    let target = root.path().to_str().expect("root");
    home.configure(BUFFERED);
    let result = home.invoke(&["guard", target, "--json"]);
    assert_eq!(result.status.code(), Some(1));
    assert_eq!(home.faults(), vec![Value::from("guard.integration-branch")]);
}

#[test]
fn erred() {
    let home = Home::new();
    let root = tempfile::tempdir().expect("root");
    let target = root.path().to_str().expect("root");
    let args = [
        "guard", target, "--base", "base", "--head", "head", "--write", "src", "--json",
    ];
    let baseline = home.invoke(&args);
    home.configure(BUFFERED);
    let result = home.invoke(&args);
    assert_eq!(result.status.code(), Some(1));
    assert_eq!(result.status.code(), baseline.status.code());
    assert_eq!(result.stdout, baseline.stdout);
    let report: Value = serde_json::from_slice(&result.stdout).expect("refusal");
    assert_eq!(report["refusal"]["kind"], "oid");
    assert_eq!(home.faults(), vec![Value::from("oid")]);
}

#[test]
fn judged() {
    let home = Home::new();
    let root = tempfile::tempdir().expect("root");
    let target = root.path().to_str().expect("root");
    home.configure(BUFFERED);
    let result = home.invoke(&["doctor", target, "--json"]);
    assert_eq!(result.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&result.stdout).expect("report");
    let first = report["findings"]
        .as_array()
        .expect("findings")
        .iter()
        .find(|finding| finding["grade"] == "out of true" || finding["grade"] == "blind")
        .expect("failing finding")["code"]
        .clone();
    assert_eq!(home.faults(), vec![first]);
}

#[test]
fn intact() {
    let home = Home::new();
    let root = tempfile::tempdir().expect("root");
    let target = root.path().to_str().expect("root");
    let args = ["doctor", target, "--json"];
    let baseline = home.invoke(&args);
    for config in [BUFFERED, "[locus]\nenabled = true\nendpoint = ''\n"] {
        home.configure(config);
        let result = home.invoke(&args);
        assert_eq!(result.status.code(), baseline.status.code());
        assert_eq!(result.stdout, baseline.stdout);
    }
}

#[test]
fn parent() {
    let home = Home::new();
    let baseline = home.run("node.version");
    home.configure(BUFFERED);
    let result = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .env("PLUMB_HOME", home.path())
        .env_remove("LOCUS_API")
        .env(locus::collector::PARENT, "")
        .args(["metadata", "node.version"])
        .output()
        .expect("command");
    assert_eq!(result.status.code(), baseline.status.code());
    assert_eq!(result.stdout, baseline.stdout);
    let diagnostic = String::from_utf8_lossy(&result.stderr);
    assert!(
        diagnostic.contains("plumb locus:") && diagnostic.contains("collector"),
        "invalid LOCUS_PARENT stays on the observation diagnostic path: {diagnostic}"
    );
}
