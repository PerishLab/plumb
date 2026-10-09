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
elif '/check-runs' in endpoint:
    result = [{'check_runs':[json.loads((root/'guard.json').read_text())]}] if (root/'guard.json').exists() else [{'check_runs':[]}]
elif '/actions/runs/' in endpoint:
    result = json.loads((root/'run.json').read_text())
elif '/comments' in endpoint:
    comments = json.loads((root / 'comments.json').read_text())
    if payload:
        comments.append(dict(payload, user={"id":1}))
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
