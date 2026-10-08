use super::{Repo, cache, support};
use serde_json::Value;
use std::process::{Command, Output};

struct Runtime {
    root: tempfile::TempDir,
    home: tempfile::TempDir,
}

impl Runtime {
    fn new() -> Self {
        let root = cache::fixture();
        Repo::git(root.path(), &["branch", "-m", "main"]);
        Repo::git(
            root.path(),
            &[
                "remote",
                "set-url",
                "origin",
                "https://git.example.invalid/Example/runtime.git",
            ],
        );
        Repo::git(root.path(), &["commit", "-q", "-m", "fixture"]);
        Self {
            root,
            home: support::home(),
        }
    }

    fn command(&self) -> Command {
        let mut command = support::plumb();
        command
            .args(["guard", ".", "--json"])
            .current_dir(self.root.path())
            .env("PLUMB_HOME", self.home.path())
            .env("PLUMB_GUARD_STRENGTH", "full")
            .env("PLUMB_GUARD_BOUNDARY", "head");
        command
    }

    fn run(&self) -> Output {
        self.command().output().expect("runtime guard")
    }
}

fn report(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "runtime report: {error}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

#[test]
fn evidence() {
    let fixture = Runtime::new();
    let first = fixture.run();
    assert!(
        first.status.success(),
        "{}{}",
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );
    let first = report(&first);
    assert_eq!(first["schema"], "plumb.guard-runtime/v1");
    assert_eq!(first["ok"], true);
    assert_eq!(first["strength"], "full");
    assert_eq!(first["boundary"], "head");
    assert_eq!(first["guard"]["repository"], "Example/runtime");
    assert_eq!(first["guard"]["tree"].as_str().unwrap().len(), 40);
    assert_eq!(first["commit"].as_str().unwrap().len(), 40);
    assert_eq!(first["digest"].as_str().unwrap().len(), 64);
    assert_ne!(first["digest"], first["guard"]["digest"]);

    let second = fixture.run();
    assert!(second.status.success());
    assert_eq!(
        first,
        report(&second),
        "exact HEAD evidence is deterministic"
    );
}

#[test]
fn refusals() {
    let fixture = Runtime::new();

    let partial = fixture
        .command()
        .env_remove("PLUMB_GUARD_BOUNDARY")
        .output()
        .expect("partial");
    assert!(!partial.status.success());
    assert!(
        report(&partial)["message"]
            .as_str()
            .unwrap()
            .contains("must be declared together")
    );

    let unknown = fixture
        .command()
        .env("PLUMB_GUARD_STRENGTH", "quick")
        .output()
        .expect("unknown");
    assert!(!unknown.status.success());
    assert!(
        report(&unknown)["message"]
            .as_str()
            .unwrap()
            .contains("must be full")
    );

    let explicit = fixture
        .command()
        .args(["--base", "HEAD", "--head", "HEAD", "--write", "."])
        .output()
        .expect("explicit");
    assert!(!explicit.status.success());
    assert!(
        report(&explicit)["message"]
            .as_str()
            .unwrap()
            .contains("disagree")
    );

    std::fs::write(fixture.root.path().join("dirty"), "dirty\n").expect("dirty");
    let dirty = fixture.run();
    assert!(!dirty.status.success());
    assert!(
        report(&dirty)["message"]
            .as_str()
            .unwrap()
            .contains("exact clean HEAD")
    );
}

#[test]
fn neutral() {
    let fixture = Runtime::new();
    let output = fixture
        .command()
        .env_remove("PLUMB_GUARD_STRENGTH")
        .env_remove("PLUMB_GUARD_BOUNDARY")
        .env("CI", "true")
        .env("GITHUB_ACTIONS", "true")
        .output()
        .expect("local guard");
    assert!(!output.status.success(), "main still has the local refusal");
    let finding = report(&output);
    assert_eq!(finding["schema"], "plumb.guard-finding/v1");
    assert_eq!(finding["code"], "guard.integration-branch");
}

#[cfg(unix)]
fn noisy(fixture: &Runtime, mode: &str) -> tempfile::TempDir {
    use std::os::unix::fs::PermissionsExt as _;
    let root = fixture.root.path();
    std::fs::write(
        root.join("plumb.toml"),
        "[workflow.hash.guard]\nweb=['apps']\n",
    )
    .expect("web policy");
    std::fs::create_dir_all(root.join("apps/web")).expect("app");
    std::fs::write(
        root.join("apps/web/package.json"),
        r#"{"name":"noisy","scripts":{"build":"held"}}"#,
    )
    .expect("manifest");
    Repo::git(root, &["add", "."]);
    Repo::git(root, &["commit", "-q", "-m", "web"]);
    let tools = tempfile::tempdir().expect("tools");
    let tail = match mode {
        "failure" => "exit 17",
        "stream" => "sleep 1",
        _ => "exit 0",
    };
    for name in ["node", "pnpm"] {
        let path = tools.path().join(name);
        let body = format!(
            "#!/bin/sh\nif [ \"$1\" = --version ]; then echo fixture; exit 0; fi\nprintf '%s\\n' '{{\"ok\":true,\"schema\":\"child-output\"}}'\nprintf '%s\\n' 'child diagnostic' >&2\n{tail}\n"
        );
        std::fs::write(&path, body).expect("child script");
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).expect("executable");
    }
    tools
}

#[cfg(unix)]
fn command(fixture: &Runtime, tools: &tempfile::TempDir) -> Command {
    let mut paths = vec![tools.path().to_path_buf()];
    paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").expect("PATH"),
    ));
    let mut command = fixture.command();
    command.env("PATH", std::env::join_paths(paths).expect("PATH"));
    command
}

#[test]
#[cfg(unix)]
fn channels() {
    let fixture = Runtime::new();
    let tools = noisy(&fixture, "success");
    let cold = command(&fixture, &tools).output().expect("cold guard");
    assert!(
        cold.status.success(),
        "{}",
        String::from_utf8_lossy(&cold.stderr)
    );
    assert_eq!(report(&cold)["ok"], true);
    let diagnostic = String::from_utf8_lossy(&cold.stderr);
    assert!(diagnostic.contains(r#"{"ok":true,"schema":"child-output"}"#));
    assert!(diagnostic.contains("child diagnostic"));
    let cached = command(&fixture, &tools).output().expect("cached guard");
    assert!(cached.status.success());
    assert_eq!(report(&cold), report(&cached));
    assert!(!String::from_utf8_lossy(&cached.stderr).contains("child-output"));
}

#[test]
#[cfg(unix)]
fn failure() {
    let fixture = Runtime::new();
    let tools = noisy(&fixture, "failure");
    let output = command(&fixture, &tools).output().expect("failed guard");
    assert!(!output.status.success());
    let finding = report(&output);
    assert_eq!(finding["ok"], false);
    assert!(finding["message"].as_str().unwrap().contains("17"));
    let diagnostic = String::from_utf8_lossy(&output.stderr);
    assert!(diagnostic.contains("child-output"));
    assert!(diagnostic.contains("child diagnostic"));
}

#[test]
#[cfg(unix)]
fn streaming() {
    use std::io::BufRead as _;
    use std::process::Stdio;
    let fixture = Runtime::new();
    let tools = noisy(&fixture, "stream");
    let mut child = command(&fixture, &tools)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("streaming guard");
    let mut diagnostic = std::io::BufReader::new(child.stderr.take().expect("stderr"));
    let mut line = String::new();
    loop {
        line.clear();
        assert_ne!(diagnostic.read_line(&mut line).expect("live diagnostic"), 0);
        if line.contains("child-output") {
            break;
        }
    }
    assert!(child.try_wait().expect("still running").is_none());
    let output = child.wait_with_output().expect("completed guard");
    assert!(output.status.success());
    assert_eq!(report(&output)["ok"], true);
}
