use super::{Provider, issue};
use serde_json::{Value, json};
#[path = "../../../tests/support/script.rs"]
mod writer;

#[path = "idle.rs"]
mod noop;

struct Fixture {
    root: tempfile::TempDir,
    command: std::path::PathBuf,
}

impl Fixture {
    fn new(routes: Value) -> Self {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("routes.json"), routes.to_string()).unwrap();
        std::fs::write(root.path().join("comments.json"), "[]").unwrap();
        let command = root.path().join("gh");
        writer::executable(
            &command,
            r#"#!/usr/bin/env python3
import json, pathlib, sys
root = pathlib.Path(__file__).parent
args = sys.argv[1:]
assert args[0] == 'api', args
endpoint = args[1]
payload = json.loads(pathlib.Path(args[args.index('--input') + 1]).read_text()) if '--input' in args else None
with (root / 'calls').open('a') as trace:
    trace.write(json.dumps({'endpoint': endpoint, 'payload': payload, 'args': args}) + '\n')
if '/comments' in endpoint:
    comments = json.loads((root / 'comments.json').read_text())
    if payload:
        comments.append(dict(payload, user={'id': 1}))
        (root / 'comments.json').write_text(json.dumps(comments))
        result = payload
    else:
        result = [comments]
else:
    result = json.loads((root / 'routes.json').read_text())[endpoint]
print(json.dumps(result))
"#,
        );
        Self { root, command }
    }

    fn provider(&self) -> Provider<'_> {
        Provider {
            command: &self.command,
            repository: "Example/probe",
        }
    }

    fn calls(&self) -> Vec<Value> {
        std::fs::read_to_string(self.root.path().join("calls"))
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }
}

fn check(conclusion: &str) -> Value {
    json!({"name":"Guard", "app":{"slug":"github-actions"}, "head_sha":"exact", "status":"completed", "conclusion":conclusion, "details_url":"https://github.com/Example/probe/actions/runs/17"})
}

fn routes(runs: Vec<Value>) -> Value {
    json!({"repos/Example/probe/commits/exact/check-runs?per_page=100":[{"check_runs":runs}],
        "repos/Example/probe/actions/runs/17":{"head_sha":"exact", "path":".github/workflows/guard.yml", "event":"pull_request"}})
}

#[test]
fn guard() {
    let success = Fixture::new(routes(vec![check("success")]));
    assert_eq!(
        success.provider().guard("exact").unwrap(),
        "https://github.com/Example/probe/actions/runs/17"
    );
    let failed = Fixture::new(routes(vec![check("failure")]));
    assert!(
        failed
            .provider()
            .guard("exact")
            .unwrap_err()
            .contains("failed")
    );
    let missing = Fixture::new(routes(vec![]));
    assert!(
        missing
            .provider()
            .guard("exact")
            .unwrap_err()
            .contains("pending")
    );
    let duplicate = Fixture::new(routes(vec![check("success"), check("success")]));
    let error = duplicate.provider().guard("exact").unwrap_err();
    assert!(error.contains("multiple"));
    assert!(!error.contains("pending"));
    let mut unreadable = check("failure");
    unreadable["conclusion"] = Value::Null;
    let error = Fixture::new(routes(vec![unreadable]))
        .provider()
        .guard("exact")
        .unwrap_err();
    assert!(error.contains("no readable conclusion"), "{error}");
    for conclusion in ["success", "failure"] {
        let mut unrelated = routes(vec![check(conclusion)]);
        unrelated["repos/Example/probe/actions/runs/17"]["path"] = "other.yml".into();
        let error = Fixture::new(unrelated)
            .provider()
            .guard("exact")
            .unwrap_err();
        assert!(error.contains("not the organization workflow"), "{error}");
    }
}

#[test]
fn comments() {
    let fixture =
        Fixture::new(json!({"repos/Example/probe/issues/17": {"state":"closed","user":{"id":1}}}));
    let provider = fixture.provider();
    provider.comment(17, "same failure\nsecond line").unwrap();
    provider.comment(17, "same failure\nsecond line").unwrap();
    provider
        .patch(
            "repos/Example/probe/issues/17",
            &json!({"body":"accepted\nexact body", "state":"closed"}),
        )
        .unwrap();
    let calls = fixture.calls();
    let posted = |body: &str| {
        calls
            .iter()
            .filter(|call| call["payload"]["body"] == body)
            .count()
    };
    assert_eq!(posted("same failure\nsecond line"), 1);
    let patch = calls.last().unwrap();
    assert_eq!(patch["payload"]["body"], "accepted\nexact body");
    assert!(patch["args"].as_array().unwrap().contains(&json!("PATCH")));
    let foreign = json!([{"body":"foreign copy","user":{"id":2}}]);
    std::fs::write(
        fixture.root.path().join("comments.json"),
        foreign.to_string(),
    )
    .unwrap();
    provider.comment(17, "foreign copy").unwrap();
    assert_eq!(
        fixture
            .calls()
            .iter()
            .filter(|call| call["payload"]["body"] == "foreign copy")
            .count(),
        1
    );
}

#[test]
fn duplicates() {
    let fixture = Fixture::new(
        json!({"repos/Example/probe/issues?state=open&per_page=100":[[
        {"type":{"name":"Auto"},"body":issue::body("Example/probe"),"number":17},
        {"type":{"name":"Auto"},"body":issue::body("Example/probe"),"number":18}]]}),
    );
    assert!(
        issue::find(&fixture.provider())
            .unwrap_err()
            .contains("multiple")
    );
    assert!(fixture.calls().iter().all(|call| call["payload"].is_null()));
}

#[path = "recovery.rs"]
mod recovery;

#[path = "candidate/stale.rs"]
mod stale;

fn isolated(name: &str) -> bool {
    if std::env::var("PLUMB_TEST_RECOVERY").as_deref() == Ok(name) {
        return true;
    }
    let home = tempfile::tempdir().unwrap();
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            &format!("command::packages::follow::tests::recovery::{name}"),
            "--nocapture",
        ])
        .env("PLUMB_TEST_RECOVERY", name)
        .env("PLUMB_TEST_HOME", home.path())
        .env("PLUMB_HOME", home.path())
        .env_remove("CI");
    for (key, _) in std::env::vars() {
        let ambient = key.starts_with("CARGO_") || key.starts_with("RUST");
        if ambient && !["CARGO_HOME", "RUSTUP_HOME", "RUSTUP_TOOLCHAIN"].contains(&key.as_str()) {
            command.env_remove(key);
        }
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"));
    false
}

#[test]
fn identity() {
    if std::env::var("PLUMB_TEST_WRITER").as_deref() != Ok("isolated") {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "command::packages::follow::tests::identity",
                "--nocapture",
            ])
            .env("PLUMB_TEST_WRITER", "isolated");
        for key in [
            "GIT_AUTHOR_NAME",
            "GIT_AUTHOR_EMAIL",
            "GIT_COMMITTER_NAME",
            "GIT_COMMITTER_EMAIL",
        ] {
            command.env_remove(key);
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    plumb::depot::carry(crate::catalog::carried::FILES);
    let fixture = Fixture::new(json!({}));
    let root = fixture.repository("pub fn answer() -> u8 { 42 }\n");
    let git = super::work::git;
    for key in ["user.name", "user.email"] {
        git(&root, &["config", key, ""]).unwrap();
    }
    let tree = git(&root, &["rev-parse", "HEAD^{tree}"]).unwrap();
    let head = git(
        &root,
        &[
            "commit-tree",
            &tree,
            "-p",
            "HEAD",
            "-m",
            "Auto base refresh",
        ],
    )
    .unwrap();
    assert_eq!(
        git(&root, &["show", "-s", "--format=%an|%ae|%cn|%ce", &head]).unwrap(),
        "Plumb Auto|auto@plumb.invalid|Plumb Auto|auto@plumb.invalid"
    );
    assert_eq!(git(&root, &["config", "user.name"]).unwrap(), "");
    assert_eq!(git(&root, &["config", "user.email"]).unwrap(), "");
}

#[test]
fn parallel() {
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(concurrent);
        }
    });
}

fn concurrent() {
    let body = format!("#!/bin/sh\nprintf fixture\n#{}\n", "x".repeat(131072));
    for _ in 0..128 {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("probe");
        writer::executable(&path, &body);
        let output = std::process::Command::new(&path).output().unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, b"fixture");
        let fixture = Fixture::new(routes(vec![check("success")]));
        assert_eq!(
            fixture.provider().guard("exact").unwrap(),
            "https://github.com/Example/probe/actions/runs/17"
        );
    }
}
