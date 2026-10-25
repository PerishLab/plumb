use super::super::super::provider::{Patience, Provider};
use super::super::super::{Input, resume, retire};
use super::{Fixture, Seat, State, isolated, work};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const FRESH: &str = r#"#!/usr/bin/env python3
import json, os, pathlib, sys
root = pathlib.Path(__file__).parent
args = sys.argv[1:]
if args[:2] != ['api', 'repos/Example/probe/issues'] or '--input' not in args:
    os.execv(str(root / 'gh'), [str(root / 'gh'), *args])
payload = json.loads(pathlib.Path(args[args.index('--input') + 1]).read_text())
held = json.loads((root / 'issue.json').read_text())
number = held['number'] + 2
fresh = dict(held, number=number, title=payload['title'], body=payload['body'], type={'name': payload['type']}, state='open', node_id='fixture-fresh', html_url=f'https://github.com/Example/probe/issues/{number}')
(root / 'issue.json').write_text(json.dumps(fresh))
(root / 'comments.json').write_text('[]')
(root / 'pull.json').replace(root / 'retired.json')
print(json.dumps(fresh))
"#;

fn provider(command: &Path) -> Provider<'_> {
    Provider {
        command,
        repository: "Example/probe",
        patience: Patience {
            deadline: std::time::Duration::from_secs(5),
            pause: std::time::Duration::from_millis(50),
        },
        sources: Default::default(),
    }
}

fn published(fixture: &Fixture, worktree: PathBuf) -> (PathBuf, State) {
    let source = fixture.repository("pub fn answer() -> u8 {\n    42\n}\n");
    let mut state = State {
        schema: "plumb.auto-state/v1".into(),
        repository: "Example/probe".into(),
        issue: 17,
        worktree,
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
    std::fs::write(path, format!("{text}description='closed by hand'\n")).unwrap();
    work::git(&state.worktree, &["commit", "-qam", "Refs #17."]).unwrap();
    let head = work::git(&state.worktree, &["rev-parse", "HEAD"]).unwrap();
    let publication = super::super::super::pull::Publication {
        root: &state.worktree,
        head: &head,
        previous: None,
    };
    fixture.provider().publish(17, publication).unwrap();
    (state.pushed, state.candidate, state.pull) = (Some(head.clone()), Some(head), 18);
    (source, state)
}

fn close(fixture: &Fixture, pull: Value) {
    let path = fixture.root.path().join("pull.json");
    let mut held: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    held.as_object_mut()
        .unwrap()
        .extend(pull.as_object().unwrap().clone());
    std::fs::write(path, held.to_string()).unwrap();
}

fn shut(fixture: &Fixture) {
    let issue = "repos/Example/probe/issues/17";
    fixture
        .provider()
        .patch(issue, &json!({"state":"closed"}))
        .unwrap();
}

fn branch(root: &Path, name: &str) -> String {
    work::git(root, &["for-each-ref", "--format=%(objectname)", name]).unwrap()
}

#[test]
fn fresh() {
    if !isolated("retire::fresh") {
        return;
    }
    plumb::depot::carry(crate::catalog::carried::FILES);
    let fixture = Fixture::new(json!({}));
    let seat = Seat::open("Example/probe").unwrap();
    let (source, mut state) = published(&fixture, seat.worktree().unwrap().join("worktree"));
    let old = state.worktree.clone();
    seat.write(&state).unwrap();
    close(&fixture, json!({"state":"closed"}));
    shut(&fixture);
    let input = Input {
        root: source.clone(),
        github: fixture.command.clone(),
        json: true,
    };
    let error = resume(&input, &fixture.provider(), &seat, &mut state).unwrap_err();
    assert!(error.contains("closed without merge"), "{error}");
    assert!(error.contains("run plumb follow --retire"), "{error}");
    let error = fixture.provider().stopped(&state, error);
    assert!(error.contains("plumb follow --retire"), "{error}");
    let comments = std::fs::read_to_string(fixture.root.path().join("comments.json")).unwrap();
    assert!(comments.contains("Auto follow stopped: Auto pull was closed without merge"));
    assert!(comments.contains("run plumb follow --retire"), "{comments}");
    let tree = work::git(&source, &["rev-parse", "HEAD^{tree}"]).unwrap();
    drop(crate::command::guard::precommit::tree::Index::new(&source, &tree).unwrap());
    let held = retire::run(&source, &fixture.provider()).unwrap_err();
    assert!(
        held.contains("follow already holds this repository"),
        "{held}"
    );
    assert!(fixture.provider().remote(17).unwrap().is_some());
    drop(seat);
    let message = retire::run(&source, &fixture.provider()).unwrap();
    assert!(message.contains("Auto #17 retired"), "{message}");
    assert!(fixture.provider().remote(17).unwrap().is_none());
    assert!(branch(&source, "refs/heads/auto/17").is_empty());
    assert!(!old.exists());
    let listed = work::git(&source, &["worktree", "list", "--porcelain"]).unwrap();
    assert!(!listed.contains("auto/17"), "{listed}");
    let locked = listed
        .lines()
        .filter(|line| line.starts_with("locked"))
        .collect::<Vec<_>>();
    let reason = format!("locked {}", plumb::integration::staging::REASON);
    assert_eq!(locked, [reason.as_str()], "{listed}");
    let seat = Seat::open("Example/probe").unwrap();
    let mut state = seat.read("Example/probe").unwrap();
    assert_eq!((state.issue, state.pull), (0, 0));
    let text = std::fs::read_to_string(source.join("plumb.toml")).unwrap();
    std::fs::write(
        source.join("plumb.toml"),
        format!("{text}\n[release]\nproduct = \"probe\"\n"),
    )
    .unwrap();
    work::git(&source, &["commit", "-qam", "stale identity"]).unwrap();
    work::git(&source, &["push", "-q", "origin", "main"]).unwrap();
    let main = work::git(&source, &["rev-parse", "HEAD"]).unwrap();
    std::fs::write(fixture.root.path().join("mergefault"), "once").unwrap();
    let command = fixture.root.path().join("fresh");
    super::super::writer::executable(&command, FRESH);
    let error = resume(&input, &provider(&command), &seat, &mut state).unwrap_err();
    assert!(
        error.contains("fixture reached verified merge boundary"),
        "{error}"
    );
    assert_eq!(
        (state.issue, state.pull, state.worktree.clone()),
        (19, 20, old)
    );
    let candidate = provider(&command).remote(19).unwrap().unwrap();
    assert_eq!(state.pushed.as_deref(), Some(candidate.as_str()));
    assert_eq!(
        work::git(&source, &["rev-parse", &format!("{candidate}^")]).unwrap(),
        main
    );
    assert!(fixture.provider().remote(17).unwrap().is_none());
}

#[test]
fn refused() {
    if !isolated("retire::refused") {
        return;
    }
    type Setup = fn(&Fixture, &Path, &State);
    let cases: [(&str, Setup); 5] = [
        ("Auto issue #17 is open", |_, _, _| {}),
        ("Auto pull #18 is open", |fixture, _, _| shut(fixture)),
        ("Auto pull #18 is merged", |fixture, _, _| {
            shut(fixture);
            close(
                fixture,
                json!({"state":"closed","merged_at":"2026-10-09T00:00:00Z"}),
            );
        }),
        ("recorded absorption", |fixture, source, state| {
            shut(fixture);
            close(fixture, json!({"state":"closed"}));
            let receipt = json!({"schema":"plumb.auto-absorbed/v1","repository":"Example/probe","issue":17,"pull":18,"head":state.pushed,"main":work::git(source, &["rev-parse", "origin/main"]).unwrap(),"resolution":plumb::packages::locked(source, "follow").unwrap()});
            let body = format!("<!-- plumb.auto-absorbed/v1\n{receipt}\n-->");
            fixture.provider().comment(17, &body).unwrap();
        }),
        (
            "moved outside the recorded Auto",
            |fixture, source, state| {
                shut(fixture);
                close(fixture, json!({"state":"closed"}));
                let head = state.pushed.as_deref().unwrap();
                let tree = format!("{head}^{{tree}}");
                let foreign =
                    work::git(source, &["commit-tree", &tree, "-p", head, "-m", "peer"]).unwrap();
                work::git(
                    source,
                    &[
                        "push",
                        "-q",
                        "origin",
                        &format!("{foreign}:refs/heads/auto/17"),
                    ],
                )
                .unwrap();
            },
        ),
    ];
    for (reason, setup) in cases {
        let fixture = Fixture::new(json!({}));
        let seat = Seat::fixture(fixture.root.path());
        let (source, mut state) = published(&fixture, fixture.root.path().join("worktree"));
        seat.write(&state).unwrap();
        setup(&fixture, &source, &state);
        let remote = fixture.provider().remote(17).unwrap();
        let error = retire::retire(&source, &fixture.provider(), &seat, &mut state).unwrap_err();
        assert!(error.starts_with("retire refused: "), "{reason}: {error}");
        assert!(error.contains(reason), "{reason}: {error}");
        assert_eq!(seat.read("Example/probe").unwrap().issue, 17, "{reason}");
        assert_eq!(fixture.provider().remote(17).unwrap(), remote, "{reason}");
        assert!(state.worktree.exists(), "{reason}");
        assert!(
            !branch(&source, "refs/heads/auto/17").is_empty(),
            "{reason}"
        );
    }
}

#[test]
fn unpulled() {
    let fixture = Fixture::new(json!({}));
    let source = fixture.repository("pub fn answer() -> u8 {\n    42\n}\n");
    let seat = Seat::fixture(fixture.root.path());
    let mut state = State {
        schema: "plumb.auto-state/v1".into(),
        repository: "Example/probe".into(),
        issue: 17,
        worktree: fixture.root.path().join("worktree"),
        ..Default::default()
    };
    seat.write(&state).unwrap();
    shut(&fixture);
    let input = Input {
        root: source.clone(),
        github: fixture.command.clone(),
        json: true,
    };
    let error = resume(&input, &fixture.provider(), &seat, &mut state).unwrap_err();
    assert!(
        error.contains("closed without a verified merged pull"),
        "{error}"
    );
    assert!(error.contains("run plumb follow --retire"), "{error}");
    let message = retire::retire(&source, &fixture.provider(), &seat, &mut state).unwrap();
    assert!(
        message.contains("Auto #17 retired: issue closed and no pull"),
        "{message}"
    );
    assert_eq!(seat.read("Example/probe").unwrap().issue, 0);
    let error = retire::retire(&source, &fixture.provider(), &seat, &mut state).unwrap_err();
    assert!(error.contains("no Auto is recorded"), "{error}");
}
