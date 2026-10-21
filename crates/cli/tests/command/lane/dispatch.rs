use super::{fixture, git};
use serde_json::{Value, json};
use std::process::{Command, Output};

struct Seat {
    product: tempfile::TempDir,
    home: tempfile::TempDir,
    config: tempfile::TempDir,
}

impl Seat {
    fn new() -> Self {
        let held = Self {
            product: fixture(),
            home: tempfile::tempdir().unwrap(),
            config: super::super::configuration::support::home(),
        };
        let mode = json!({"login":"LiberteCode", "control":"f".repeat(40),
            "commit":git(held.product.path(), &["rev-parse","HEAD"]), "tree":git(held.product.path(), &["rev-parse","HEAD^{tree}"]), "fail":false});
        held.mode(&mode);
        plumb::testing::executable(
            &held.home.path().join("gh"),
            r#"#!/usr/bin/env python3
import json,os,pathlib,sys
home=pathlib.Path(os.environ['HOME'])
mode=json.loads((home/'mode.json').read_text())
if mode.get('overflow'):
 print('x'*65537)
 sys.exit(0)
with (home/'calls.jsonl').open('a') as f:
 f.write(json.dumps({'args':sys.argv[1:],'provider':os.environ.get('CLOUDFLARE_API_TOKEN'),'packages':os.environ.get('WHARF_PACKAGES_TOKEN')})+'\n')
if 'POST' in sys.argv:
 body=json.load(sys.stdin)
 with (home/'posts.jsonl').open('a') as f:f.write(json.dumps(body)+'\n')
 if mode['fail']:
  print('private-provider-value',file=sys.stderr)
  sys.exit(1)
 sys.exit(0)
if 'user' in sys.argv: print(json.dumps({'login':mode['login']}))
elif 'repos/PerishLab/wharf/commits/main' in sys.argv:print(json.dumps({'sha':mode['control']}))
else:print(json.dumps({'sha':mode['commit'],'tree':mode['tree']}))
"#,
        );
        let source =
            json!({"commit": mode["commit"], "tree":mode["tree"], "declaration":"d".repeat(64)});
        let document = json!({"schema":"wharf.lane.dispatch/v1", "target":{"repository":"PerishLab/crest","app":"crest","lane":"design"},
            "operation":"apply", "request":"a".repeat(64), "revision":1,"expected":{"state":"absent"},"source":source,
            "registration":"e".repeat(64),"caller":"LiberteCode","workflow":format!("PerishLab/wharf/.github/workflows/lane.yml@{}","f".repeat(40))});
        held.write("input.json", &document);
        held
    }

    fn mode(&self, value: &Value) {
        self.write("mode.json", value);
    }
    fn write(&self, name: &str, value: &Value) {
        std::fs::write(
            self.home.path().join(name),
            serde_json::to_vec(value).unwrap(),
        )
        .unwrap();
    }
    fn read(&self, name: &str) -> Value {
        serde_json::from_slice(&std::fs::read(self.home.path().join(name)).unwrap()).unwrap()
    }
    fn path(&self, name: &str) -> String {
        self.home.path().join(name).to_str().unwrap().into()
    }
    fn run(&self, arguments: &[&str]) -> Output {
        let path = std::env::join_paths(
            std::iter::once(self.home.path().to_path_buf())
                .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
        )
        .unwrap();
        Command::new(env!("CARGO_BIN_EXE_plumb"))
            .args(arguments)
            .env("PATH", path)
            .env("HOME", self.home.path())
            .env("PLUMB_HOME", self.config.path())
            .env("CLOUDFLARE_API_TOKEN", "private-provider-value")
            .env("WHARF_PACKAGES_TOKEN", "private-packages-value")
            .output()
            .unwrap()
    }
    fn prepare(&self) -> Value {
        let output = self.run(&[
            "lane",
            "plan",
            self.product.path().to_str().unwrap(),
            "--app",
            "crest",
            "--lane",
            "design",
            "--input",
            &self.path("input.json"),
            "--json",
        ]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let value = serde_json::from_slice(&output.stdout).unwrap();
        self.write("plan.json", &value);
        value
    }
    fn execute(&self, command: &str, digest: &str) -> Output {
        self.run(&[
            "lane",
            command,
            self.product.path().to_str().unwrap(),
            "--plan",
            &self.path("plan.json"),
            "--digest",
            digest,
            "--json",
        ])
    }
    fn posts(&self) -> Vec<Value> {
        std::fs::read_to_string(self.home.path().join("posts.jsonl"))
            .unwrap_or_default()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }
}

fn success(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn submit() {
    let seat = Seat::new();
    let index = std::fs::read(seat.product.path().join(".git/index")).unwrap();
    let report = seat.prepare();
    assert_eq!(report["current"]["outcome"], "unknown");
    assert!(seat.posts().is_empty());
    let output = seat.execute("apply", report["digest"].as_str().unwrap());
    let result = success(&output);
    assert_eq!(result["submission"], "acknowledged");
    assert_eq!(result["operation"]["outcome"], "unknown");
    assert_eq!(result["current"]["outcome"], "unknown");
    let posts = seat.posts();
    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0]["ref"], "main");
    assert_eq!(
        serde_json::from_str::<Value>(posts[0]["inputs"]["request"].as_str().unwrap()).unwrap(),
        seat.read("input.json")
    );
    for line in std::fs::read_to_string(seat.home.path().join("calls.jsonl"))
        .unwrap()
        .lines()
    {
        let call: Value = serde_json::from_str(line).unwrap();
        assert!(call["provider"].is_null());
        assert!(call["packages"].is_null());
    }
    assert_eq!(
        std::fs::read(seat.product.path().join(".git/index")).unwrap(),
        index
    );
    assert!(
        !seat
            .product
            .path()
            .join("apps/review/SHOULD_NOT_EXIST")
            .exists()
    );
}

#[test]
fn moved() {
    for key in ["login", "control", "tree", "commit"] {
        let seat = Seat::new();
        let report = seat.prepare();
        let mut mode = seat.read("mode.json");
        mode[key] = json!("moved");
        seat.mode(&mode);
        let output = seat.execute("apply", report["digest"].as_str().unwrap());
        assert!(!output.status.success(), "{key}");
        assert!(output.stdout.is_empty());
        assert!(seat.posts().is_empty());
    }
}

#[test]
fn review() {
    let seat = Seat::new();
    let report = seat.prepare();
    let digest = report["digest"].as_str().unwrap();
    let output = seat.execute("discard", digest);
    assert!(!output.status.success());
    assert!(seat.posts().is_empty());
    let output = seat.execute("apply", &"0".repeat(64));
    assert!(!output.status.success());
    assert!(seat.posts().is_empty());
    let mut changed = report.clone();
    changed["plan"]["dispatch"]["request"] = json!("b".repeat(64));
    seat.write("plan.json", &changed);
    let output = seat.execute("apply", digest);
    assert!(!output.status.success());
    assert!(seat.posts().is_empty());
    seat.write("plan.json", &report);
    std::fs::write(seat.product.path().join("changed"), "preserve").unwrap();
    let output = seat.execute("apply", digest);
    assert!(!output.status.success());
    assert!(seat.posts().is_empty());
    git(seat.product.path(), &["add", "."]);
    git(
        seat.product.path(),
        &["commit", "--quiet", "-m", "source moved"],
    );
    let output = seat.execute("apply", digest);
    assert!(!output.status.success());
    assert!(seat.posts().is_empty());
}

#[test]
fn uncertain() {
    let seat = Seat::new();
    let report = seat.prepare();
    let mut mode = seat.read("mode.json");
    mode["fail"] = json!(true);
    seat.mode(&mode);
    let output = seat.execute("apply", report["digest"].as_str().unwrap());
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("outcome unknown"));
    assert!(!error.contains("private-provider-value"));
    assert_eq!(seat.posts().len(), 1);
}

#[test]
fn discard() {
    let seat = Seat::new();
    let mut input = seat.read("input.json");
    input["operation"] = json!("dispose");
    input["source"] = Value::Null;
    input["revision"] = json!(2);
    input["expected"] = json!({"state":"present","revision":1,"digest":"1".repeat(64)});
    seat.write("input.json", &input);
    let report = seat.prepare();
    let output = seat.execute("discard", report["digest"].as_str().unwrap());
    success(&output);
    assert_eq!(seat.posts().len(), 1);
    let output = seat.execute("apply", report["digest"].as_str().unwrap());
    assert!(!output.status.success());
    assert_eq!(seat.posts().len(), 1);
}

#[test]
fn budget() {
    let seat = Seat::new();
    let report = seat.prepare();
    let mut mode = seat.read("mode.json");
    mode["overflow"] = json!(true);
    seat.mode(&mode);
    let output = seat.execute("apply", report["digest"].as_str().unwrap());
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(seat.posts().is_empty());
}
