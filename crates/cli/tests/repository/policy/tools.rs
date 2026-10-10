use std::path::Path;
use std::process::Command;

struct Fixture {
    root: tempfile::TempDir,
    home: tempfile::TempDir,
}

impl Fixture {
    fn new(paths: &[&str]) -> Self {
        let root = tempfile::tempdir().expect("repository");
        let home = tempfile::tempdir().expect("rules home");
        for path in paths {
            std::fs::create_dir_all(root.path().join(path)).expect("source seat");
        }
        super::govern(root.path());
        let rules = rules();
        let scan = std::fs::read_to_string(rules.join("atoms/scan.toml")).expect("scan rules");
        let shape = std::fs::read_to_string(rules.join("suites/shape.toml")).expect("shape rules");
        super::stock(home.path(), super::LIMIT, &scan, &shape);
        Self { root, home }
    }

    fn call(&self, verb: &str) -> std::process::Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_plumb"));
        command.arg(verb).arg(self.root.path());
        command.env("PLUMB_HOME", self.home.path());
        if verb == "policy" {
            command.arg("--write");
        }
        command.output().expect("plumb runs")
    }

    fn policy(&self) -> toml::Value {
        let result = self.call("policy");
        assert!(result.status.success(), "{result:?}");
        let path = self.root.path().join("ectropy.toml");
        let text = std::fs::read_to_string(path).expect("rendered policy");
        let judged = self.call("doctor");
        let output = String::from_utf8_lossy(&judged.stdout);
        assert!(!output.contains("missing ectropy"), "{output}");
        assert!(!output.contains("unexpected ectropy"), "{output}");
        toml::from_str(&text).expect("policy parses")
    }
}

fn rules() -> std::path::PathBuf {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .expect("git runs");
    assert!(output.status.success());
    let root = String::from_utf8(output.stdout).expect("git path");
    Path::new(root.trim()).join("crates/cli/rules")
}

fn strings(value: &toml::Value) -> Vec<&str> {
    value
        .as_array()
        .expect("list")
        .iter()
        .map(|item| item.as_str().expect("path"))
        .collect()
}

fn grants<'a>(policy: &'a toml::Value, syntax: &str) -> Vec<&'a str> {
    policy["grant"]
        .as_array()
        .expect("grants")
        .iter()
        .filter(|entry| entry["syntax"].as_str() == Some(syntax))
        .flat_map(|entry| strings(&entry["paths"]))
        .collect()
}

#[test]
fn absent() {
    let fixture = Fixture::new(&["crates/engine/src"]);
    let policy = fixture.policy();
    assert_eq!(strings(&policy["scan"]["include"]), ["crates/**/*.rs"]);
    assert!(!strings(&policy["module"]["roots"]).contains(&"tools/*/src"));
}

#[test]
fn tools() {
    let fixture = Fixture::new(&["tools/check/src", "tools/check/tests"]);
    let policy = fixture.policy();
    assert_eq!(strings(&policy["scan"]["include"]), ["tools/**/*.rs"]);
    assert_eq!(
        strings(&policy["module"]["roots"]),
        ["tools/*/src", "tools/*/tests"]
    );
    assert_eq!(
        grants(&policy, "test"),
        ["tools/*/src/**/*.rs", "tools/*/tests/**/*.rs"]
    );
    assert_eq!(grants(&policy, "environment"), ["tools/*/tests/**/*.rs"]);
    assert_eq!(policy["limit"]["block"].as_integer(), Some(4));
    assert_eq!(policy["limit"]["path"].as_integer(), Some(3));
}

#[test]
fn mixed() {
    let fixture = Fixture::new(&["crates/engine/src", "tools/check/tests"]);
    let policy = fixture.policy();
    assert_eq!(
        strings(&policy["scan"]["include"]),
        ["crates/**/*.rs", "tools/**/*.rs"]
    );
    assert_eq!(
        grants(&policy, "environment"),
        ["crates/*/tests/**/*.rs", "tools/*/tests/**/*.rs"]
    );
    assert_eq!(strings(&policy["scan"]["exclude"]), ["**/target/**"]);
}
