#[cfg(unix)]
use super::Client;
use super::{parse, pull};
#[cfg(unix)]
use crate::delivery::{Patience, Squash};
#[cfg(unix)]
use serde_json::{Value, json};
#[cfg(unix)]
use std::path::Path;
#[cfg(unix)]
use std::time::Duration;

#[test]
fn remotes() {
    for raw in [
        "git@github.com:PerishLab/plumb.git",
        "ssh://git@github.com/PerishLab/plumb.git",
        "https://github.com/PerishLab/plumb",
    ] {
        let held = parse(raw).expect(raw);
        assert_eq!(
            (held.owner.as_str(), held.repo.as_str()),
            ("PerishLab", "plumb")
        );
    }
    let refused = parse("ssh://git@git.perish.top/PerishLab/plumb.git").err();
    assert!(refused.is_some_and(|error| error.contains("Forgejo is archived")));
}

#[test]
fn pulls() {
    let value = serde_json::json!({
        "id": "PR_one",
        "number": 23,
        "url": "https://github.com/PerishLab/plumb/pull/23",
        "headRefOid": "abc",
        "title": "Delivery",
        "body": "Refs PerishLab/plumb#20"
    });
    let pull = pull(&value).expect("pull");
    assert_eq!(pull.node, "PR_one");
    assert_eq!(pull.number, 23);
}

#[cfg(unix)]
const HEAD: &str = "c0ffee0000000000000000000000000000000458";

#[cfg(unix)]
const GH: &str = r#"#!/usr/bin/env python3
import json, os, sys
root = os.path.dirname(os.path.abspath(__file__))
config = json.load(open(os.path.join(root, "config.json")))
path = os.path.join(root, "state.json")
state = json.load(open(path)) if os.path.exists(path) else {"checks": 0, "runs": 0, "views": 0, "merges": 0, "settled": False}
args = sys.argv[1:]
with open(os.path.join(root, "calls.jsonl"), "a") as log:
    log.write(json.dumps(args) + "\n")
def done(value=None, code=0, error=""):
    json.dump(state, open(path, "w"))
    if value is not None:
        print(json.dumps(value))
    if error:
        print(error, file=sys.stderr)
    sys.exit(code)
head = config["head"]
if args[:1] == ["api"] and "/check-runs" in args[1]:
    state["checks"] += 1
    pending = state["checks"] <= config["check_pending"]
    done([{"check_runs": [{"name": "Guard", "app": {"slug": "github-actions"}, "head_sha": head,
        "status": "in_progress" if pending else "completed",
        "conclusion": None if pending else "success",
        "details_url": "https://github.com/PerishLab/plumb/actions/runs/77/job/1"}]}])
if args[:1] == ["api"] and args[1].endswith("/actions/runs/77"):
    state["runs"] += 1
    pending = state["runs"] <= config["run_pending"]
    if not pending and config["run_conclusion"] == "success":
        state["settled"] = True
    done({"head_sha": head, "path": ".github/workflows/guard.yml", "event": "pull_request",
        "status": "in_progress" if pending else "completed",
        "conclusion": None if pending else config["run_conclusion"]})
if args[:2] == ["pr", "view"]:
    views = config["views"]
    status = views[min(state["views"], len(views) - 1)]
    state["views"] += 1
    done({"headRefOid": config.get("view_head", head), "mergeStateStatus": status})
if args[:2] == ["pr", "merge"]:
    state["merges"] += 1
    if not state["settled"]:
        done(code=3, error="merged before the required Guard workflow run completed")
    if state["merges"] <= config["refusals"]:
        done(code=1, error="X Pull request PerishLab/plumb#458 is not mergeable: the base branch policy prohibits the merge.")
    done()
done(code=2, error="unexpected gh call: " + " ".join(args))
"#;

#[cfg(unix)]
fn client(root: &Path, config: Value) -> Client {
    std::fs::write(root.join("config.json"), config.to_string()).unwrap();
    let command = root.join("gh");
    crate::testing::executable(&command, GH);
    Client {
        seat: "PerishLab/plumb".into(),
        command,
        patience: Patience {
            deadline: Duration::from_secs(5),
            pause: Duration::from_millis(20),
        },
    }
}

#[cfg(unix)]
fn squash() -> Squash {
    Squash {
        candidate: HEAD.into(),
        subject: "Wait for Guard (#458)".into(),
        body: "Refs PerishLab/plumb#458".into(),
    }
}

#[cfg(unix)]
fn calls(root: &Path) -> Vec<Vec<String>> {
    std::fs::read_to_string(root.join("calls.jsonl"))
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[cfg(unix)]
fn merges(root: &Path) -> Vec<Vec<String>> {
    calls(root)
        .into_iter()
        .filter(|call| call[..2] == ["pr", "merge"])
        .collect()
}

#[cfg(unix)]
#[test]
fn ruleset() {
    let dir = tempfile::tempdir().unwrap();
    let client = client(
        dir.path(),
        json!({"head": HEAD, "check_pending": 2, "run_pending": 2, "run_conclusion": "success",
            "views": ["BLOCKED", "CLEAN"], "refusals": 1}),
    );
    client.settle(458, &squash()).expect("settled merge");
    let merges = merges(dir.path());
    assert_eq!(
        merges.len(),
        2,
        "one policy refusal retried once: {merges:?}"
    );
    for merge in &merges {
        assert!(
            merge
                .windows(2)
                .any(|pair| pair == ["--match-head-commit", HEAD])
        );
        assert!(merge.iter().any(|arg| arg == "--squash"));
        assert!(!merge.iter().any(|arg| arg == "--admin" || arg == "--auto"));
    }
    let calls = calls(dir.path());
    let first = calls
        .iter()
        .position(|call| call[..2] == ["pr", "merge"])
        .unwrap();
    let runs = calls[..first]
        .iter()
        .filter(|call| call[0] == "api" && call[1].ends_with("/actions/runs/77"))
        .count();
    assert_eq!(runs, 3, "merge only after the workflow run read completed");
    assert!(
        calls[..first]
            .iter()
            .any(|call| call[..2] == ["pr", "view"])
    );
}

#[cfg(unix)]
#[test]
fn stalled() {
    let failed = tempfile::tempdir().unwrap();
    let error = client(
        failed.path(),
        json!({"head": HEAD, "check_pending": 0, "run_pending": 0, "run_conclusion": "failure",
            "views": ["CLEAN"], "refusals": 0}),
    )
    .settle(458, &squash())
    .unwrap_err();
    assert!(error.contains("organization Guard failed"), "{error}");
    assert!(merges(failed.path()).is_empty());

    let blocked = tempfile::tempdir().unwrap();
    let mut held = client(
        blocked.path(),
        json!({"head": HEAD, "check_pending": 0, "run_pending": 0, "run_conclusion": "success",
            "views": ["BLOCKED"], "refusals": 0}),
    );
    held.patience.deadline = Duration::from_millis(300);
    let error = held.settle(458, &squash()).unwrap_err();
    assert!(
        error.contains("bounded wait") && error.contains("BLOCKED"),
        "{error}"
    );
    assert!(merges(blocked.path()).is_empty());

    let dirty = tempfile::tempdir().unwrap();
    let error = client(
        dirty.path(),
        json!({"head": HEAD, "check_pending": 0, "run_pending": 0, "run_conclusion": "success",
            "views": ["DIRTY"], "refusals": 0}),
    )
    .settle(458, &squash())
    .unwrap_err();
    assert!(error.contains("mergeStateStatus DIRTY"), "{error}");
    let views = calls(dirty.path())
        .into_iter()
        .filter(|call| call[..2] == ["pr", "view"])
        .count();
    assert_eq!(views, 1);
    assert!(merges(dirty.path()).is_empty());

    let moved = tempfile::tempdir().unwrap();
    let error = client(
        moved.path(),
        json!({"head": HEAD, "check_pending": 0, "run_pending": 0, "run_conclusion": "success",
            "views": ["CLEAN"], "refusals": 0, "view_head": "f00"}),
    )
    .settle(458, &squash())
    .unwrap_err();
    assert!(error.contains("not the exact candidate"), "{error}");
    assert!(merges(moved.path()).is_empty());
}
