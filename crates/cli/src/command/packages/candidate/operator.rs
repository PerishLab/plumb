use super::{Fixture, State, input, publish, work};
use serde_json::{Value, json};
use std::path::Path;

fn stage(fixture: &Fixture, source: &Path, stage: &str) -> State {
    let mut state = State {
        schema: "plumb.auto-state/v1".into(),
        repository: "Example/probe".into(),
        issue: 17,
        ..Default::default()
    };
    if stage == "noop" {
        return state;
    }
    let head = publish(fixture, source);
    state.pushed = Some(head.clone());
    if stage == "publish" {
        return state;
    }
    let merged = stage == "merged";
    let pull = json!({"number":18, "state": if merged {"closed"} else {"open"},
        "head":{"ref":"auto/17","sha":head,"repo":{"full_name":"Example/probe"}},
        "base":{"ref":"main"}, "body":"Refs #17.\n",
        "merged_at": if merged {json!("2026-10-09T00:00:00Z")} else {Value::Null},
        "merge_commit_sha": head});
    std::fs::write(fixture.root.path().join("pull.json"), pull.to_string()).unwrap();
    if stage == "absorbed" {
        let main = format!("{head}:refs/heads/main");
        work::git(source, &["push", "-q", "origin", &main]).unwrap();
    }
    state.pull = 18;
    state
}

#[test]
fn operator() {
    for name in ["publish", "absorbed", "noop", "merged"] {
        for adopted in [true, false] {
            let fixture = Fixture::new(json!({}));
            let source = fixture.repository("pub fn answer() -> u8 { 42 }\n");
            let held = stage(&fixture, &source, name);
            let (source, seat) = fixture.cold(&source);
            if !adopted {
                seat.write(&held).unwrap();
            }
            std::fs::write(fixture.root.path().join("viewer"), "foreign-operator").unwrap();
            let provider = fixture.provider();
            let remote = provider.remote(17).unwrap();
            let before = fixture.calls().len();
            let mut state = seat.read("Example/probe").unwrap();
            let error = super::super::super::super::resume(
                &input(&fixture, &source),
                &provider,
                &seat,
                &mut state,
            )
            .unwrap_err();
            let error = provider.stopped(&state, error);
            assert!(
                error.contains(
                    "operation writer fixture-writer, but this follow runs as foreign-operator"
                ),
                "{name} {adopted}: {error}"
            );
            let calls = fixture.calls();
            assert!(
                calls[before..].iter().all(|call| call["payload"].is_null()),
                "{name}"
            );
            assert_eq!(provider.remote(17).unwrap(), remote, "{name}");
            let branch = work::git(&source, &["show-ref", "--verify", "refs/heads/auto/17"]);
            assert!(branch.is_err(), "{name}");
            let trees = work::git(&source, &["worktree", "list", "--porcelain"]).unwrap();
            assert_eq!(trees.matches("worktree ").count(), 1, "{name}");
            assert!(
                !seat.worktree().unwrap().join("worktree").exists(),
                "{name}"
            );
            let seated = seat.read("Example/probe").unwrap();
            assert_eq!(
                (seated.issue, seated.pull),
                if adopted { (0, 0) } else { (17, held.pull) }
            );
        }
    }
}
