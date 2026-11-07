use super::super::{State, retire, work};
use super::Fixture;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

fn commit(root: &Path, text: &str, message: &str) -> String {
    std::fs::write(root.join("Cargo.lock"), text).unwrap();
    work::git(root, &["add", "Cargo.lock"]).unwrap();
    work::git(root, &["commit", "-q", "-m", message]).unwrap();
    work::git(root, &["rev-parse", "HEAD"]).unwrap()
}

fn repository(fixture: &Fixture) -> PathBuf {
    let origin = fixture.root.path().join("origin.git");
    let source = fixture.root.path().join("source");
    work::git(fixture.root.path(), &["init", "-q", "--bare", "origin.git"]).unwrap();
    work::git(fixture.root.path(), &["init", "-q", "-b", "main", "source"]).unwrap();
    work::git(
        &source,
        &["remote", "add", "origin", origin.to_str().unwrap()],
    )
    .unwrap();
    commit(&source, "base\n", "base");
    work::git(&source, &["push", "-q", "origin", "main"]).unwrap();
    source
}

fn absorbed(source: &Path, number: u64) -> String {
    let branch = format!("auto/{number}");
    work::git(source, &["checkout", "-q", "-b", &branch, "origin/main"]).unwrap();
    let tip = commit(
        source,
        &format!("follow {number}\n"),
        "Follow the latest first-party packages",
    );
    work::git(source, &["push", "-q", "origin", &branch]).unwrap();
    work::git(source, &["checkout", "-q", "main"]).unwrap();
    commit(
        source,
        &format!("follow {number}\n"),
        &format!("Follow (#{})", number + 1),
    );
    work::git(source, &["push", "-q", "origin", "main"]).unwrap();
    work::git(source, &["branch", "-q", "-D", &branch]).unwrap();
    work::git(source, &["fetch", "-q", "origin"]).unwrap();
    tip
}

fn routes(number: u64, issue: &str, pull: Value) -> Value {
    json!({
        format!("repos/Example/probe/issues/{number}"): {"number":number, "state":issue},
        format!("repos/Example/probe/pulls?state=all&head=Example:auto/{number}&per_page=100"): [[pull]],
    })
}

fn merged(number: u64) -> Value {
    json!({"number":number + 1, "state":"closed", "merged_at":"2026-11-05T18:23:34Z", "head":{"ref":format!("auto/{number}")}})
}

fn refused(fixture: &Fixture, source: &Path, state: &State, expected: &str) {
    let error = retire::Leftover {
        root: source,
        provider: &fixture.provider(),
        state,
    }
    .check()
    .unwrap_err();
    assert!(error.contains(expected), "{error}");
}

#[test]
fn dynet() {
    let fixture = Fixture::new(routes(134, "closed", merged(134)));
    let source = repository(&fixture);
    retire::Leftover {
        root: &source,
        provider: &fixture.provider(),
        state: &State::default(),
    }
    .check()
    .unwrap();
    assert!(!fixture.root.path().join("calls").exists());
    let tip = absorbed(&source, 134);
    assert_eq!(
        work::Origin(&source).cherry("origin/main", &tip).unwrap(),
        vec![('-', tip.clone())]
    );
    assert!(
        work::Origin(&source)
            .merges(&format!("origin/main..{tip}"))
            .unwrap()
            .is_empty()
    );
    assert_eq!(work::Origin(&source).heads().unwrap(), vec![(134, tip)]);
    retire::Leftover {
        root: &source,
        provider: &fixture.provider(),
        state: &State::default(),
    }
    .check()
    .unwrap();
    let running = State {
        issue: 134,
        ..State::default()
    };
    refused(&fixture, &source, &running, "running follow");
}

#[test]
fn provider() {
    let mut open = merged(134);
    open["state"] = json!("open");
    open["merged_at"] = Value::Null;
    let mut unmerged = merged(134);
    unmerged["merged_at"] = Value::Null;
    for (routes, expected) in [
        (routes(134, "open", merged(134)), "not a closed issue"),
        (routes(134, "closed", open), "open pull request"),
        (routes(134, "closed", unmerged), "found 0"),
    ] {
        let fixture = Fixture::new(routes);
        let source = repository(&fixture);
        absorbed(&source, 134);
        refused(&fixture, &source, &State::default(), expected);
    }
}

#[test]
fn history() {
    let fixture = Fixture::new(routes(134, "closed", merged(134)));
    let source = repository(&fixture);
    work::git(
        &source,
        &["checkout", "-q", "-b", "auto/134", "origin/main"],
    )
    .unwrap();
    commit(&source, "unique\n", "not on main");
    work::git(&source, &["push", "-q", "origin", "auto/134"]).unwrap();
    work::git(&source, &["checkout", "-q", "main"]).unwrap();
    refused(&fixture, &source, &State::default(), "not marked -");
    work::git(&source, &["checkout", "-q", "-b", "side", "main"]).unwrap();
    work::git(&source, &["commit", "-q", "--allow-empty", "-m", "side"]).unwrap();
    work::git(&source, &["checkout", "-q", "auto/134"]).unwrap();
    work::git(&source, &["merge", "-q", "--no-ff", "-m", "merge", "side"]).unwrap();
    work::git(&source, &["push", "-q", "origin", "auto/134"]).unwrap();
    work::git(&source, &["checkout", "-q", "main"]).unwrap();
    refused(&fixture, &source, &State::default(), "merge commit");
    work::git(&source, &["push", "-q", "-f", "origin", "main:auto/134"]).unwrap();
    refused(&fixture, &source, &State::default(), "is empty");
    work::git(&source, &["push", "-q", "origin", "main:auto/stray"]).unwrap();
    refused(&fixture, &source, &State::default(), "no Issue number");
}
