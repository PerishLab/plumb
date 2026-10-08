use super::super::{Seat, State, closure, engine, work};
use super::{Fixture, issue};
use serde_json::json;

impl Fixture {
    fn repository(&self, code: &str) -> std::path::PathBuf {
        let source = self.root.path().join("source");
        let origin = self.root.path().join("origin.git");
        std::fs::create_dir_all(source.join("src")).unwrap();
        for args in [
            vec!["init", "-q", "-b", "main"],
            vec!["config", "user.name", "Fixture"],
            vec!["config", "user.email", "fixture@example.invalid"],
        ] {
            work::git(&source, &args).unwrap();
        }
        std::process::Command::new("git")
            .args(["init", "-q", "--bare"])
            .arg(&origin)
            .status()
            .unwrap();
        work::git(
            &source,
            &["remote", "add", "origin", origin.to_str().unwrap()],
        )
        .unwrap();
        std::fs::write(
            source.join("Cargo.toml"),
            "[package]\nname='fixture'\nversion='0.1.0'\nedition='2024'\n",
        )
        .unwrap();
        std::fs::write(source.join("src/lib.rs"), code).unwrap();
        std::fs::write(
            source.join("plumb.toml"),
            "[workflow.hash.guard]\nrust=['Cargo.toml','Cargo.lock','src']\n",
        )
        .unwrap();
        let locked = std::process::Command::new("cargo")
            .args(["generate-lockfile", "--offline"])
            .current_dir(&source)
            .output()
            .unwrap();
        assert!(
            locked.status.success(),
            "{}",
            String::from_utf8_lossy(&locked.stderr)
        );
        work::git(&source, &["add", "."]).unwrap();
        work::git(&source, &["commit", "-q", "-m", "base"]).unwrap();
        work::git(&source, &["push", "-q", "origin", "main"]).unwrap();
        work::git(&source, &["fetch", "origin"]).unwrap();
        std::fs::write(self.root.path().join("issue.json"), json!({"number":17,"type":{"name":"Auto"},"body":issue::body("Example/probe"),"state":"open","labels":[],"node_id":"fixture-issue","html_url":"https://github.com/Example/probe/issues/17","title":"Follow","updated_at":"2026-10-08T00:00:00Z"}).to_string()).unwrap();
        std::fs::write(&self.command, r#"#!/usr/bin/env python3
import json, pathlib, subprocess, sys
root = pathlib.Path(__file__).parent
args = sys.argv[1:]
endpoint = args[1]
payload = json.loads(pathlib.Path(args[args.index('--input') + 1]).read_text()) if '--input' in args else None
with (root / 'calls').open('a') as trace:
    trace.write(json.dumps({'endpoint':endpoint,'payload':payload}) + '\n')
def head():
    return subprocess.check_output(['git','--git-dir',str(root/'origin.git'),'rev-parse','refs/heads/auto/17'], text=True).strip()
if '/issues?' in endpoint:
    result = [[]]
elif '/git/matching-refs/' in endpoint:
    probe = subprocess.run(['git','--git-dir',str(root/'origin.git'),'show-ref','--verify','--hash','refs/heads/auto/17'], capture_output=True, text=True)
    result = [[{'ref':'refs/heads/auto/17','object':{'sha':probe.stdout.strip()}}]] if probe.returncode == 0 else [[]]
elif '/git/ref/' in endpoint:
    result = {'object':{'sha':head()}}
elif '/pulls' in endpoint:
    path = root / 'pull.json'
    if payload:
        pull = {'number':18,'state':'open','head':{'ref':'auto/17','sha':head()},'base':{'ref':'main'},'body':payload['body']}
        path.write_text(json.dumps(pull))
        if (root / 'fault').exists():
            (root / 'fault').unlink()
            print('fixture interrupted after provider write', file=sys.stderr)
            sys.exit(17)
        result = pull
    else:
        result = [[json.loads(path.read_text())]] if path.exists() else [[]]
elif '/comments' in endpoint:
    comments = json.loads((root / 'comments.json').read_text())
    if payload:
        comments.append(payload)
        (root / 'comments.json').write_text(json.dumps(comments))
        result = payload
    else:
        result = [comments]
elif '/sub_issues' in endpoint or '/dependencies/' in endpoint:
    result = [[]]
else:
    result = json.loads((root / 'issue.json').read_text())
    if payload:
        result.update(payload)
        (root / 'issue.json').write_text(json.dumps(result))
        if (root / 'closefault').exists():
            (root / 'closefault').unlink()
            print('fixture interrupted after closure write', file=sys.stderr)
            sys.exit(17)
print(json.dumps(result))
"#).unwrap();
        source
    }
}

#[test]
fn interrupted() {
    plumb::depot::carry(crate::catalog::carried::FILES);
    let fixture = Fixture::new(json!({}));
    let source = fixture.repository("pub fn answer() -> u8 {\n    false\n}\n");
    let seat = Seat::fixture(fixture.root.path());
    let mut state = State {
        schema: "plumb.auto-state/v1".into(),
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
    let manifest = state.worktree.join("Cargo.toml");
    let current = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(
        manifest,
        format!("{current}description='bounded fixture'\n"),
    )
    .unwrap();
    work::git(&state.worktree, &["add", "Cargo.toml"]).unwrap();
    work::git(
        &state.worktree,
        &["commit", "-q", "-m", "fixture manifest delta"],
    )
    .unwrap();
    std::fs::write(fixture.root.path().join("fault"), "once").unwrap();
    let fault = engine::advance(&source, &fixture.provider(), &seat, &mut state).unwrap_err();
    assert!(
        fault.contains("interrupted after provider write"),
        "{fault}"
    );
    state = seat.read("Example/probe").unwrap();
    assert_eq!(state.pull, 0);
    assert!(
        state.candidate.is_some(),
        "candidate is durable before provider write"
    );
    for _ in 0..2 {
        let error = engine::advance(&source, &fixture.provider(), &seat, &mut state).unwrap_err();
        assert!(error.contains("failed with"), "{error}");
        state = seat.read("Example/probe").unwrap();
        assert_eq!(state.pull, 18);
        assert!(state.merged.is_none());
    }
    assert_eq!(fixture.provider().issue(17).unwrap()["state"], "open");
    assert_eq!(
        fixture
            .calls()
            .iter()
            .filter(|call| call["endpoint"] == "repos/Example/probe/pulls"
                && !call["payload"].is_null())
            .count(),
        1
    );
}

#[test]
fn closed() {
    plumb::depot::carry(crate::catalog::carried::FILES);
    let fixture = Fixture::new(json!({}));
    let source = fixture.repository("pub fn answer() -> u8 {\n    42\n}\n");
    let seat = Seat::fixture(fixture.root.path());
    let mut state = State {
        schema: "plumb.auto-state/v1".into(),
        repository: "Example/probe".into(),
        issue: 17,
        pull: 18,
        worktree: fixture.root.path().join("worktree"),
        ..Default::default()
    };
    work::Work {
        source: &source,
        state: &state,
    }
    .open()
    .unwrap();
    let manifest = state.worktree.join("Cargo.toml");
    let current = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(
        manifest,
        format!("{current}description='bounded fixture'\n"),
    )
    .unwrap();
    work::git(&state.worktree, &["add", "Cargo.toml"]).unwrap();
    work::git(
        &state.worktree,
        &["commit", "-q", "-m", "fixture manifest delta"],
    )
    .unwrap();
    let proof = crate::command::guard::precommit::branch::renew(&state.worktree).unwrap();
    let parent = work::git(&source, &["rev-parse", "origin/main"]).unwrap();
    let token = proof.encode().unwrap();
    let message = format!("Refs #17.\n\n{} {token}", plumb::guard::TRAILER);
    let candidate = work::git(
        &source,
        &["commit-tree", &proof.tree, "-p", &parent, "-m", &message],
    )
    .unwrap();
    let merged = work::git(
        &source,
        &[
            "commit-tree",
            &proof.tree,
            "-p",
            &parent,
            "-m",
            &format!("Fixture merge\n\n{} {token}", plumb::guard::TRAILER),
        ],
    )
    .unwrap();
    work::git(
        &source,
        &[
            "push",
            "origin",
            &format!("{candidate}:refs/heads/auto/17"),
            &format!("{merged}:refs/heads/main"),
        ],
    )
    .unwrap();
    state.candidate = Some(candidate);
    state.merged = Some(merged);
    state.guard = Some("https://github.com/Example/probe/actions/runs/17".into());
    seat.write(&state).unwrap();
    let owned = state.worktree.clone();
    std::fs::write(fixture.root.path().join("closefault"), "once").unwrap();
    assert!(
        closure::close(&source, &fixture.provider(), &seat, &mut state)
            .unwrap_err()
            .contains("interrupted after closure write")
    );
    state = seat.read("Example/probe").unwrap();
    closure::close(&source, &fixture.provider(), &seat, &mut state).unwrap();
    let issue = fixture.provider().issue(17).unwrap();
    assert_eq!(issue["state"], "closed");
    assert_eq!(
        issue["body"],
        issue::body("Example/probe").replace("- [ ]", "- [x]")
    );
    assert_eq!(seat.read("Example/probe").unwrap().issue, 0);
    assert!(!owned.exists());
    let comments: Vec<serde_json::Value> =
        serde_json::from_slice(&std::fs::read(fixture.root.path().join("comments.json")).unwrap())
            .unwrap();
    assert_eq!(
        comments
            .iter()
            .filter(|comment| comment["body"]
                .as_str()
                .unwrap()
                .contains("plumb.auto-closure/v1"))
            .count(),
        1
    );
    assert!(fixture.provider().remote(17).unwrap().is_none());
}

#[test]
fn current() {
    plumb::depot::carry(crate::catalog::carried::FILES);
    let fixture = Fixture::new(json!({}));
    let source = fixture.repository("pub fn answer() -> u8 {\n    42\n}\n");
    let seat = Seat::fixture(fixture.root.path());
    let mut state = State {
        schema: "plumb.auto-state/v1".into(),
        repository: "Example/probe".into(),
        ..Default::default()
    };
    let input = super::super::Input {
        root: source.clone(),
        github: fixture.command.clone(),
        json: true,
    };
    let result = super::super::resume(&input, &fixture.provider(), &seat, &mut state).unwrap();
    assert!(result.contains("already current"));
    assert_eq!(state.issue, 0);
    assert!(fixture.calls().iter().all(|call| call["payload"].is_null()));
    assert_eq!(work::git(&source, &["status", "--porcelain"]).unwrap(), "");
    assert!(!fixture.root.path().join("pull.json").exists());
}
