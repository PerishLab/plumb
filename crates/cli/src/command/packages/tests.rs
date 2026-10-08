use super::{Provider, issue};
use serde_json::{Value, json};
use std::os::unix::fs::PermissionsExt;

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
        std::fs::write(&command, r#"#!/usr/bin/env python3
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
        comments.append(payload)
        (root / 'comments.json').write_text(json.dumps(comments))
        result = payload
    else:
        result = [comments]
else:
    result = json.loads((root / 'routes.json').read_text())[endpoint]
print(json.dumps(result))
"#).unwrap();
        std::fs::set_permissions(&command, std::fs::Permissions::from_mode(0o755)).unwrap();
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
    let mut unrelated = routes(vec![check("success")]);
    unrelated["repos/Example/probe/actions/runs/17"]["path"] = "other.yml".into();
    assert!(Fixture::new(unrelated).provider().guard("exact").is_err());
}

#[test]
fn comments() {
    let fixture = Fixture::new(json!({"repos/Example/probe/issues/17": {"state":"closed"}}));
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
    assert_eq!(
        calls
            .iter()
            .filter(|call| call["payload"]["body"] == "same failure\nsecond line")
            .count(),
        1
    );
    assert_eq!(calls[3]["payload"]["body"], "accepted\nexact body");
    assert!(
        calls[3]["args"]
            .as_array()
            .unwrap()
            .contains(&json!("PATCH"))
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
