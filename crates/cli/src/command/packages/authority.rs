use super::{Fixture, State, isolated, issue, work};
use plumb::guard::{Authority, Expected};
use serde_json::json;

#[test]
fn proof() {
    if !isolated("authority::proof") {
        return;
    }
    plumb::depot::carry(crate::catalog::carried::FILES);
    let fixture = Fixture::new(json!({}));
    let source = fixture.repository("pub fn answer() -> u8 {\n    42\n}\n");
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
    let root = &state.worktree;
    let manifest = root.join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(
        manifest,
        format!("{text}description='registered fixture delta'\n"),
    )
    .unwrap();
    work::git(root, &["add", "Cargo.toml"]).unwrap();
    work::git(root, &["commit", "-q", "-m", "fixture delta"]).unwrap();
    let proof = crate::command::guard::precommit::branch::renew(root).unwrap();
    let head = work::git(root, &["rev-parse", "HEAD"]).unwrap();
    let authority = Authority::running().unwrap();
    let expected = Expected::new(&proof.schema, &proof.tree, &proof.digest);
    authority.verify(root, &head, &expected).unwrap();
    assert_eq!(proof.plumb, authority.producer());
    assert_eq!(proof.depot, authority.depot());
    let wrong = Expected::new(&proof.schema, "0".repeat(40), &proof.digest);
    assert!(authority.verify(root, &head, &wrong).is_err());
    for field in ["producer", "depot"] {
        let mut foreign = proof.clone();
        if field == "producer" {
            foreign.plumb = "foreign".into();
        } else {
            foreign.depot = "0".repeat(64);
        }
        let foreign = foreign
            .resolved(plumb::packages::locked(root, "local-locked").unwrap())
            .unwrap();
        let message = format!(
            "Adversarial fixture\n\n{} {}",
            plumb::guard::TRAILER,
            foreign.encode().unwrap()
        );
        let candidate = work::git(
            root,
            &["commit-tree", &foreign.tree, "-p", &head, "-m", &message],
        )
        .unwrap();
        let expected = Expected::new(&foreign.schema, &foreign.tree, &foreign.digest);
        assert!(authority.verify(root, &candidate, &expected).is_err());
    }
    let snapshot = issue::snapshot(&fixture.provider(), 17).unwrap();
    let narrative = plumb::delivery::Narrative {
        title: "Follow first-party fixture packages".into(),
        body: "Refs #17. Registered fixture delta with exact actual local Guard.".into(),
    };
    let plan = plumb::delivery::prepare(
        plumb::delivery::Request {
            root,
            repository: "Example/probe",
            issue: &snapshot,
            observed: 1,
            base: "main",
            pull: &narrative,
        },
        &authority,
    )
    .unwrap();
    assert_eq!(plan.source, head);
    assert_eq!(plan.guard.digest, proof.digest);
    assert_eq!(
        work::git(
            root,
            &["rev-parse", &format!("{}^{{tree}}", plan.candidate)]
        )
        .unwrap(),
        proof.tree
    );
    assert!(fixture.calls().iter().all(|call| call["payload"].is_null()));
}

#[test]
fn cold() {
    if !isolated("authority::cold") {
        return;
    }
    plumb::depot::carry(crate::catalog::carried::FILES);
    let fixture = Fixture::new(json!({}));
    let source = fixture.repository("pub fn answer() -> u8 {\n    42\n}\n");
    let manifest = source.join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(manifest, format!("{text}description='cold merge'\n")).unwrap();
    work::git(&source, &["add", "Cargo.toml"]).unwrap();
    work::git(&source, &["commit", "-q", "-m", "Refs #17."]).unwrap();
    let proof = crate::command::guard::precommit::branch::renew(&source).unwrap();
    let parent = work::git(&source, &["rev-parse", "origin/main"]).unwrap();
    let message = format!(
        "Refs #17.\n\n{} {}",
        plumb::guard::TRAILER,
        proof.encode().unwrap()
    );
    let candidate = work::git(
        &source,
        &["commit-tree", &proof.tree, "-p", &parent, "-m", &message],
    )
    .unwrap();
    let merge = work::git(
        &source,
        &[
            "commit-tree",
            &proof.tree,
            "-p",
            &parent,
            "-m",
            &format!("Merged\n{message}"),
        ],
    )
    .unwrap();
    work::git(
        &source,
        &[
            "push",
            "origin",
            &format!("{merge}:refs/heads/main"),
            &format!("{candidate}:refs/pull/18/head"),
        ],
    )
    .unwrap();
    std::fs::write(fixture.root.path().join("pull.json"), json!({"number":18,"state":"closed","merged_at":"2026-10-09T00:00:00Z","merge_commit_sha":merge,"head":{"ref":"auto/17","sha":candidate,"repo":{"full_name":"Example/probe"}},"base":{"ref":"main"},"body":"Refs #17."}).to_string()).unwrap();
    std::fs::write(fixture.root.path().join("guard.json"), json!({"name":"Guard","app":{"slug":"github-actions"},"head_sha":candidate,"status":"completed","conclusion":"success","details_url":"https://github.com/Example/probe/actions/runs/17"}).to_string()).unwrap();
    std::fs::write(
        fixture.root.path().join("run.json"),
        json!({"head_sha":candidate,"path":".github/workflows/guard.yml","event":"pull_request"})
            .to_string(),
    )
    .unwrap();
    let (source, seat) = fixture.cold(&source);
    assert!(work::git(&source, &["cat-file", "-e", &candidate]).is_err());
    assert!(fixture.provider().remote(17).unwrap().is_none());
    let input = super::super::super::Input {
        root: source.clone(),
        github: fixture.command.clone(),
        json: true,
    };
    let mut state = seat.read("Example/probe").unwrap();
    let result =
        super::super::super::resume(&input, &fixture.provider(), &seat, &mut state).unwrap();
    assert!(result.contains("completed via pull #18"), "{result}");
    assert_eq!(fixture.provider().issue(17).unwrap()["state"], "closed");
    assert_eq!(state.issue, 0);
    let result =
        super::super::super::resume(&input, &fixture.provider(), &seat, &mut state).unwrap();
    assert!(result.contains("already current"), "{result}");
    assert_eq!(
        fixture
            .calls()
            .iter()
            .filter(|call| call["endpoint"] == "repos/Example/probe/pulls"
                && !call["payload"].is_null())
            .count(),
        0
    );
}

#[test]
fn issueonly() {
    plumb::depot::carry(crate::catalog::carried::FILES);
    let fixture = Fixture::new(json!({}));
    let source = fixture.repository("pub fn answer() -> u8 { 42 }\n");
    std::fs::write(source.join("package.json"), "{").unwrap();
    work::git(&source, &["add", "package.json"]).unwrap();
    work::git(&source, &["commit", "-q", "-m", "fixture resolver blocker"]).unwrap();
    work::git(&source, &["push", "origin", "main"]).unwrap();
    let (source, seat) = fixture.cold(&source);
    let input = super::super::super::Input {
        root: source.clone(),
        github: fixture.command.clone(),
        json: true,
    };
    let mut state = seat.read("Example/probe").unwrap();
    let error =
        super::super::super::resume(&input, &fixture.provider(), &seat, &mut state).unwrap_err();
    assert!(error.contains("EOF"), "{error}");
    assert_eq!(state.issue, 17);
    assert!(!fixture.root.path().join("pull.json").exists());
    assert_eq!(fixture.provider().issue(17).unwrap()["state"], "open");
    assert!(fixture.calls().iter().all(|call| call["payload"].is_null()));
}

#[test]
fn stopped() {
    for reason in ["failure", "closed"] {
        let fixture = Fixture::new(json!({}));
        let source = fixture.repository("pub fn answer() -> u8 { 42 }\n");
        let head = work::git(&source, &["rev-parse", "HEAD"]).unwrap();
        fixture
            .provider()
            .publish(
                17,
                super::super::super::pull::Publication {
                    root: &source,
                    head: &head,
                    previous: None,
                },
            )
            .unwrap();
        stop(&fixture, reason, &head);
        let (source, seat) = fixture.cold(&source);
        let before = fixture.calls().len();
        let input = super::super::super::Input {
            root: source.clone(),
            github: fixture.command.clone(),
            json: true,
        };
        let mut state = seat.read("Example/probe").unwrap();
        let error = super::super::super::resume(&input, &fixture.provider(), &seat, &mut state)
            .unwrap_err();
        let expected = if reason == "failure" {
            "Guard failed"
        } else {
            "closed without merge"
        };
        assert!(error.contains(expected), "{reason}: {error}");
        assert!(
            fixture.calls()[before..]
                .iter()
                .all(|call| call["payload"].is_null())
        );
    }
}

fn stop(fixture: &Fixture, reason: &str, head: &str) {
    if reason == "failure" {
        std::fs::write(fixture.root.path().join("guard.json"), json!({"name":"Guard","app":{"slug":"github-actions"},"head_sha":head,"status":"completed","conclusion":"failure","details_url":"https://github.com/Example/probe/actions/runs/17"}).to_string()).unwrap();
    } else {
        let path = fixture.root.path().join("pull.json");
        let mut pull: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        pull["state"] = "closed".into();
        std::fs::write(path, pull.to_string()).unwrap();
    }
}
