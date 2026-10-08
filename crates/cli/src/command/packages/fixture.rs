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
