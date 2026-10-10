use super::{Gate, Plan, confirm, prepare, revalidate};
use crate::delivery::{Narrative, Request, Snapshot, Squash};

#[path = "fixture.rs"]
mod fixture;

use fixture::{Check, Repo, git};

fn issue() -> Snapshot {
    Snapshot {
        node: "I_native".to_string(),
        repository: "PerishLab/probe".to_string(),
        number: 1,
        url: "https://github.com/PerishLab/probe/issues/1".to_string(),
        title: "Native delivery".to_string(),
        state: "OPEN".to_string(),
        kind: "Feature".to_string(),
        updated: "2026-09-30T00:00:00Z".to_string(),
        parent: None,
        sub_issues: Vec::new(),
        blocked_by: Vec::new(),
        blocking: Vec::new(),
    }
}

fn story() -> Narrative {
    Narrative {
        title: "Native delivery".to_string(),
        body: "Refs PerishLab/probe#1".to_string(),
    }
}

fn request<'a>(repo: &'a Repo, issue: &'a Snapshot, pull: &'a Narrative) -> Request<'a> {
    Request {
        root: repo.root(),
        repository: "PerishLab/probe",
        issue,
        observed: 1,
        base: "main",
        pull,
    }
}

fn plan(repo: &Repo, gate: &impl Gate) -> Plan {
    prepare(request(repo, &issue(), &story()), gate).expect("native plan")
}

#[test]
fn exact() {
    let repo = Repo::new();
    let gate = Check::new("ok");
    let plan = plan(&repo, &gate);
    assert_eq!(plan.source, git(repo.root(), &["rev-parse", "HEAD"]));
    assert_eq!(plan.target, git(repo.root(), &["rev-parse", "origin/main"]));
    assert_eq!(
        plan.evidence.tree,
        git(
            repo.root(),
            &["rev-parse", &format!("{}^{{tree}}", plan.candidate)]
        )
    );
    assert_eq!(gate.calls.get(), 1);
    let current =
        revalidate(request(&repo, &issue(), &story()), &plan, &gate).expect("fresh check");
    assert_eq!(gate.calls.get(), 2);
    assert_eq!(current, plan);
    let message = git(repo.root(), &["log", "-1", "--format=%B", &plan.candidate]);
    assert!(!message.contains(crate::guard::TRAILER));
    assert!(message.contains(super::evidence::TRAILER));
    let merged = repo.merge(&plan.candidate, &message);
    let landed = super::landed(repo.root(), &plan.candidate, &merged).expect("native readback");
    assert_eq!(landed.tree, plan.evidence.tree);
    assert!(crate::delivery::landed(repo.root(), &plan.candidate, &merged).is_err());
    assert_eq!(
        Squash::read(repo.root(), &plan.candidate)
            .expect("squash")
            .candidate,
        plan.candidate
    );
    let json = serde_json::to_vec(&plan).expect("JSON");
    assert_eq!(serde_json::from_slice::<Plan>(&json).expect("plan"), plan);
}

#[test]
fn refusal() {
    for event in [
        "failure",
        "source",
        "tree",
        "authority",
        "digest",
        "base",
        "branch",
        "dirty",
        "head",
    ] {
        let repo = Repo::new();
        let gate = Check::new(event);
        assert!(
            prepare(request(&repo, &issue(), &story()), &gate).is_err(),
            "{event}"
        );
        assert_eq!(gate.calls.get(), 1);
    }
}

#[test]
fn governed() {
    for location in ["source", "base"] {
        let repo = Repo::new();
        repo.commit("plumb.toml", "[layout]\n");
        if location == "base" {
            git(
                repo.root(),
                &["update-ref", "refs/remotes/origin/main", "HEAD"],
            );
            git(repo.root(), &["rm", "plumb.toml"]);
            git(repo.root(), &["commit", "-q", "-m", "Remove governance"]);
        }
        let gate = Check::new("ok");
        assert!(prepare(request(&repo, &issue(), &story()), &gate).is_err());
        assert_eq!(gate.calls.get(), 0);
    }
    let repo = Repo::new();
    git(
        repo.root(),
        &[
            "commit",
            "-q",
            "--amend",
            "-m",
            "guarded\n\nPlumb-Guard-Proof: invalid",
        ],
    );
    assert!(prepare(request(&repo, &issue(), &story()), &Check::new("ok")).is_err());
}

#[test]
fn stale() {
    let repo = Repo::new();
    let gate = Check::new("ok");
    let plan = plan(&repo, &gate);
    let refused = |issue: &Snapshot, pull: &Narrative, held: &Plan| {
        revalidate(request(&repo, issue, pull), held, &gate).is_err()
            && confirm(request(&repo, issue, pull), held, &gate).is_err()
    };
    let mut issue = issue();
    issue.updated.push_str("changed");
    assert!(refused(&issue, &story(), &plan));
    let mut pull = story();
    pull.body.push_str(" changed");
    assert!(refused(&plan.issue, &pull, &plan));
    let mut changed = plan.clone();
    changed.evidence.digest = "2".repeat(64);
    assert!(refused(&plan.issue, &story(), &changed));
    changed = plan.clone();
    changed.evidence.authority = "unaccepted".to_string();
    assert!(refused(&plan.issue, &story(), &changed));
    repo.commit("new", "new");
    assert!(refused(&plan.issue, &story(), &plan));
}

#[test]
fn confirmed() {
    let repo = Repo::new();
    let gate = Check::new("ok");
    let plan = plan(&repo, &gate);
    let current =
        confirm(request(&repo, &plan.issue, &story()), &plan, &gate).expect("identity check");
    assert_eq!(current, plan);
    assert_eq!((gate.calls.get(), gate.identified.get()), (1, 1));
    let drift = Check::new("identity");
    assert!(confirm(request(&repo, &plan.issue, &story()), &plan, &drift).is_err());
    assert_eq!((drift.calls.get(), drift.identified.get()), (0, 1));
    let mut unknown = plan.clone();
    unknown.schema.push_str("-unknown");
    assert!(confirm(request(&repo, &plan.issue, &story()), &unknown, &gate).is_err());
    assert_eq!((gate.calls.get(), gate.identified.get()), (1, 1));
}

#[test]
fn drifted() {
    for event in ["base", "branch", "dirty", "head"] {
        let repo = Repo::new();
        let plan = plan(&repo, &Check::new("ok"));
        let gate = Check::new(event);
        assert!(
            confirm(request(&repo, &plan.issue, &story()), &plan, &gate).is_err(),
            "{event}"
        );
        assert_eq!((gate.calls.get(), gate.identified.get()), (0, 1));
    }
}

#[test]
fn projection() {
    let repo = Repo::new();
    let source = git(repo.root(), &["rev-parse", "HEAD"]);
    git(repo.root(), &["checkout", "-q", "main"]);
    repo.commit("newbase", "new base");
    git(
        repo.root(),
        &["update-ref", "refs/remotes/origin/main", "HEAD"],
    );
    git(repo.root(), &["checkout", "-q", "topic"]);
    let gate = Check::new("ok");
    assert!(prepare(request(&repo, &issue(), &story()), &gate).is_err());
    assert_eq!(gate.calls.get(), 0);
    git(repo.root(), &["merge", "-q", "--no-edit", "main"]);
    let plan = plan(&repo, &gate);
    assert_ne!(plan.source, source);
    assert_eq!(gate.calls.get(), 1);
}

#[test]
fn readback() {
    let repo = Repo::new();
    let plan = plan(&repo, &Check::new("ok"));
    let message = git(repo.root(), &["log", "-1", "--format=%B", &plan.candidate]);
    let missing = repo.merge(&plan.candidate, "Lost evidence");
    assert!(super::landed(repo.root(), &plan.candidate, &missing).is_err());
    let duplicated = repo.merge(
        &plan.candidate,
        &format!("{message}\n{} invalid", super::evidence::TRAILER),
    );
    assert!(super::landed(repo.root(), &plan.candidate, &duplicated).is_err());
    let mixed = repo.merge(
        &plan.candidate,
        &format!("{message}\nPlumb-Guard-Proof: invalid"),
    );
    assert!(super::landed(repo.root(), &plan.candidate, &mixed).is_err());
    let tree = git(repo.root(), &["rev-parse", "origin/main^{tree}"]);
    let wrong = git(
        repo.root(),
        &["commit-tree", &tree, "-p", &plan.target, "-m", &message],
    );
    assert!(super::landed(repo.root(), &plan.candidate, &wrong).is_err());
    let moved = git(
        repo.root(),
        &[
            "commit-tree",
            &plan.evidence.tree,
            "-p",
            &plan.source,
            "-m",
            &message,
        ],
    );
    assert!(super::landed(repo.root(), &plan.candidate, &moved).is_err());
}

#[test]
fn retired() {
    let repo = Repo::new();
    let plan = plan(&repo, &Check::new("ok"));
    let message = git(repo.root(), &["log", "-1", "--format=%B", &plan.candidate]);
    let head = repo.merge(&plan.candidate, &message);
    let held = super::landed(repo.root(), &plan.candidate, &head).expect("native readback");
    let origin = tempfile::tempdir().expect("origin");
    let bare = origin.path().to_str().expect("utf8");
    git(origin.path(), &["init", "-q", "--bare"]);
    git(repo.root(), &["remote", "add", "origin", bare]);
    let projection = format!("{}:refs/heads/land/topic", plan.candidate);
    git(repo.root(), &["push", "-q", "origin", "topic", &projection]);
    let pushed = [
        crate::delivery::Pushed {
            branch: "land/topic".into(),
            head: plan.candidate.clone(),
        },
        crate::delivery::Pushed {
            branch: "topic".into(),
            head: plan.source.clone(),
        },
    ];
    let retired = held.retire(repo.root(), "origin", &pushed);
    assert_eq!(retired.deleted, ["land/topic", "topic"]);
    assert!(retired.kept.is_empty(), "{:?}", retired.kept);
    assert!(git(repo.root(), &["ls-remote", "--heads", "origin"]).is_empty());
    assert_eq!(git(repo.root(), &["branch", "--list", "topic"]), "");
}
