use super::{Fixture, issue, work};
use serde_json::json;

impl Fixture {
    pub(super) fn repository(&self, code: &str) -> std::path::PathBuf {
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
        std::fs::write(self.root.path().join("issue.json"), json!({"number":17,"type":{"name":"Auto"},"body":issue::body("Example/probe"),"state":"open","labels":[],"node_id":"fixture-issue","html_url":"https://github.com/Example/probe/issues/17","title":"Follow","updated_at":"2026-10-08T00:00:00Z","user":{"id":1}}).to_string()).unwrap();
        std::fs::write(&self.command, r#"#!/usr/bin/env python3
import json, pathlib, subprocess, sys
root = pathlib.Path(__file__).parent
args = sys.argv[1:]
endpoint = args[1]
if args[:2] == ['pr', 'merge'] and (root / 'mergefault').exists():
    sys.exit('fixture reached verified merge boundary')
payload = json.loads(pathlib.Path(args[args.index('--input') + 1]).read_text()) if '--input' in args else None
with (root / 'calls').open('a') as trace:
    trace.write(json.dumps({'endpoint':endpoint,'payload':payload}) + '\n')
def head():
    return subprocess.check_output(['git','--git-dir',str(root/'origin.git'),'rev-parse','refs/heads/auto/17'], text=True).strip()
if '/issues?' in endpoint:
    issue = json.loads((root / 'issue.json').read_text())
    result = [[issue]] if (root / 'discover').exists() and issue['state'] == 'open' else [[]]
elif '/git/matching-refs/' in endpoint:
    probe = subprocess.run(['git','--git-dir',str(root/'origin.git'),'show-ref','--verify','--hash','refs/heads/auto/17'], capture_output=True, text=True)
    result = [[{'ref':'refs/heads/auto/17','object':{'sha':probe.stdout.strip()}}]] if probe.returncode == 0 else [[]]
elif '/git/ref/' in endpoint:
    if (root / 'pushfault').exists():
        (root / 'pushfault').unlink()
        sys.exit('fixture interrupted after branch push')
    result = {'object':{'sha':head()}}
elif '/pulls' in endpoint:
    path = root / 'pull.json'
    if payload:
        pull = {'number':18,'state':'open','head':{'ref':'auto/17','sha':head(),'repo':{'full_name':'Example/probe'}},'base':{'ref':'main'},'body':payload['body']}
        path.write_text(json.dumps(pull))
        if (root / 'fault').exists():
            (root / 'fault').unlink()
            print('fixture interrupted after provider write', file=sys.stderr)
            sys.exit(17)
        result = pull
    else:
        result = [[json.loads(path.read_text())]] if path.exists() else [[]]
elif '/check-runs' in endpoint and (root / 'mergefault').exists():
    result = [{'check_runs':[{'name':'Guard','app':{'slug':'github-actions'},'head_sha':head(),'status':'completed','conclusion':'success','details_url':'https://github.com/Example/probe/actions/runs/17'}]}]
elif '/check-runs' in endpoint:
    result = [{'check_runs':[json.loads((root/'guard.json').read_text())]}] if (root/'guard.json').exists() else [{'check_runs':[]}]
elif '/actions/runs/' in endpoint and (root / 'mergefault').exists():
    result = {'head_sha':head(),'path':'.github/workflows/guard.yml','event':'pull_request'}
elif '/actions/runs/' in endpoint:
    result = json.loads((root/'run.json').read_text())
elif '/comments' in endpoint:
    comments = json.loads((root / 'comments.json').read_text())
    if payload:
        comments.append(dict(payload, user={"id":1}))
        (root / 'comments.json').write_text(json.dumps(comments))
        issue = json.loads((root / 'issue.json').read_text())
        issue['updated_at'] = f'2026-10-09T00:00:{len(comments):02d}Z'
        (root / 'issue.json').write_text(json.dumps(issue))
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

impl Fixture {
    pub(super) fn cold(&self, source: &std::path::Path) -> (std::path::PathBuf, super::Seat) {
        if let Ok(home) = std::env::var("PLUMB_TEST_HOME") {
            assert_eq!(std::env::var("PLUMB_HOME").unwrap(), home);
            std::fs::remove_dir_all(&home).unwrap();
            std::fs::create_dir(&home).unwrap();
        }
        if source.parent() != Some(self.root.path()) {
            std::fs::remove_dir_all(source.parent().unwrap()).unwrap();
        } else {
            std::fs::remove_dir_all(source).unwrap();
        }
        for path in [
            self.root.path().join("worktree"),
            self.root.path().join("state.json"),
        ] {
            if path.is_dir() {
                std::fs::remove_dir_all(path).unwrap();
            } else if path.exists() {
                std::fs::remove_file(path).unwrap();
            }
        }
        let home = tempfile::Builder::new()
            .prefix("cold-")
            .tempdir_in(self.root.path())
            .unwrap()
            .keep();
        let source = home.join("source");
        let cloned = std::process::Command::new("git")
            .args(["clone", "--no-local", "--single-branch", "--branch", "main"])
            .arg(self.root.path().join("origin.git"))
            .arg(&source)
            .output()
            .unwrap();
        assert!(
            cloned.status.success(),
            "{}",
            String::from_utf8_lossy(&cloned.stderr)
        );
        work::git(&source, &["config", "user.name", "Fixture"]).unwrap();
        work::git(
            &source,
            &["config", "user.email", "fixture@example.invalid"],
        )
        .unwrap();
        std::fs::write(self.root.path().join("discover"), "enabled").unwrap();
        (source, super::Seat::fixture(&home))
    }
}

#[test]
fn publication() {
    if !super::isolated("fixture::publication") {
        return;
    }
    plumb::depot::carry(crate::catalog::carried::FILES);
    let fixture = Fixture::new(json!({}));
    let source = fixture.repository("pub fn answer() -> u8 {\n    42\n}\n");
    let seat = super::Seat::fixture(fixture.root.path());
    let mut state = super::State {
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
    std::fs::write(path, format!("{text}description='publication recovery'\n")).unwrap();
    work::git(&state.worktree, &["add", "Cargo.toml"]).unwrap();
    work::git(&state.worktree, &["commit", "-q", "-m", "Refs #17."]).unwrap();
    std::fs::write(fixture.root.path().join("mergefault"), "once").unwrap();
    let error =
        super::engine::advance(&source, &fixture.provider(), &seat, &mut state).unwrap_err();
    assert!(
        error.contains("fixture reached verified merge boundary"),
        "{error}"
    );
    assert!(state.merged.is_none());
    assert_eq!(fixture.provider().issue(17).unwrap()["state"], "open");
    assert_eq!(
        state.plan.unwrap().issue.updated,
        issue::snapshot(&fixture.provider(), 17).unwrap().updated
    );
}

#[test]
fn policy() {
    let fixture = Fixture::new(json!({}));
    fixture.repository("pub fn answer() -> u8 { 42 }\n");
    let path = fixture.root.path().join("issue.json");
    let original = std::fs::read(&path).unwrap();
    let provider = fixture.provider();
    let snapshot = issue::snapshot(&provider, 17).unwrap();
    for delta in [
        json!({"title":"foreign title"}),
        json!({"body":"foreign policy"}),
        json!({"labels":[{"name":"needs:judgment"}]}),
        json!({"state":"closed"}),
    ] {
        std::fs::write(&path, &original).unwrap();
        provider
            .patch("repos/Example/probe/issues/17", &delta)
            .unwrap();
        assert!(provider.published(&snapshot).is_err(), "{delta}");
    }
}
