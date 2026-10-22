use super::{Fixture, Seat, State, engine, isolated, work};
use serde_json::json;

fn staged(fixture: &Fixture) -> (std::path::PathBuf, Seat, State) {
    let source = fixture.repository("pub fn answer() -> u8 {\n    42\n}\n");
    let seat = Seat::fixture(fixture.root.path());
    let state = State {
        repository: "Example/probe".into(),
        issue: 17,
        worktree: fixture.root.path().join("worktree"),
        ..Default::default()
    };
    work::Work {
        source: &source,
        state: &state,
    }
    .open()
    .unwrap();
    let path = state.worktree.join("Cargo.toml");
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(path, format!("{text}description='ruleset settle'\n")).unwrap();
    work::git(&state.worktree, &["add", "Cargo.toml"]).unwrap();
    work::git(&state.worktree, &["commit", "-q", "-m", "Refs #17."]).unwrap();
    std::fs::write(fixture.root.path().join("passing"), "always").unwrap();
    (source, seat, state)
}

fn verbs(fixture: &Fixture, verb: &str) -> Vec<Vec<String>> {
    fixture
        .calls()
        .into_iter()
        .filter(|call| call["args"][0] == "pr" && call["args"][1] == verb)
        .map(|call| serde_json::from_value(call["args"].clone()).unwrap())
        .collect()
}

#[test]
fn ruleset() {
    if !isolated("merge::ruleset") {
        return;
    }
    plumb::depot::carry(crate::catalog::carried::FILES);
    let fixture = Fixture::new(json!({}));
    let (source, seat, mut state) = staged(&fixture);
    let root = fixture.root.path();
    std::fs::write(root.join("policyfault"), "once").unwrap();
    std::fs::write(
        root.join("ready.json"),
        json!(["BLOCKED", "CLEAN"]).to_string(),
    )
    .unwrap();
    let result = engine::advance(&source, &fixture.provider(), &seat, &mut state).unwrap();
    assert!(result.contains("completed via pull #18"), "{result}");
    assert!(!root.join("policyfault").exists());
    assert_eq!(fixture.provider().issue(17).unwrap()["state"], "closed");
    let pull: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("pull.json")).unwrap()).unwrap();
    assert_eq!(pull["merged"], true);
    let merge = pull["merge_commit_sha"].as_str().unwrap();
    assert!(result.ends_with(merge), "{result}");
    assert_eq!(verbs(&fixture, "view").len(), 3);
    let merges = verbs(&fixture, "merge");
    assert_eq!(merges.len(), 2);
    let tree = |commit: &str| {
        work::git(
            &root.join("origin.git"),
            &["rev-parse", &format!("{commit}^{{tree}}")],
        )
        .unwrap()
    };
    for args in merges {
        let at = args
            .iter()
            .position(|arg| arg == "--match-head-commit")
            .unwrap();
        assert_eq!(tree(&args[at + 1]), tree(merge));
        assert!(args.contains(&"--squash".to_string()));
        assert!(
            !args.iter().any(|arg| arg == "--admin" || arg == "--auto"),
            "{args:?}"
        );
    }
}

#[test]
fn stalled() {
    if !isolated("merge::stalled") {
        return;
    }
    plumb::depot::carry(crate::catalog::carried::FILES);
    let fixture = Fixture::new(json!({}));
    let (source, seat, mut state) = staged(&fixture);
    let root = fixture.root.path();
    std::fs::write(root.join("ready.json"), json!(["BLOCKED"]).to_string()).unwrap();
    let error = engine::advance(&source, &fixture.provider(), &seat, &mut state).unwrap_err();
    assert!(error.contains("bounded wait"), "{error}");
    assert!(error.contains("BLOCKED"), "{error}");
    assert!(state.merged.is_none());
    assert!(verbs(&fixture, "view").len() > 1);
    assert!(verbs(&fixture, "merge").is_empty());
    assert_eq!(fixture.provider().issue(17).unwrap()["state"], "open");
    let before = verbs(&fixture, "view").len();
    std::fs::write(root.join("ready.json"), json!(["DIRTY"]).to_string()).unwrap();
    let error = engine::advance(&source, &fixture.provider(), &seat, &mut state).unwrap_err();
    assert!(error.contains("mergeStateStatus DIRTY"), "{error}");
    assert_eq!(verbs(&fixture, "view").len(), before + 1);
    assert!(verbs(&fixture, "merge").is_empty());
    assert!(state.merged.is_none());
}
