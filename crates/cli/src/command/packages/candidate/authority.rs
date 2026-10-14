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
