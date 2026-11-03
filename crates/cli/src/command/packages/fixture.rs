use super::{Fixture, issue, work};
use serde_json::json;

impl Fixture {
    fn git(&self, root: &std::path::Path, args: &[&str]) -> Result<String, String> {
        let output = plumb::config::detached("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_AUTHOR_NAME", "Plumb Auto")
            .env("GIT_AUTHOR_EMAIL", "auto@plumb.invalid")
            .env("GIT_COMMITTER_NAME", "Plumb Auto")
            .env("GIT_COMMITTER_EMAIL", "auto@plumb.invalid")
            .stdin(std::process::Stdio::null())
            .output()
            .map_err(|error| error.to_string())?;
        if !output.status.success() {
            return Err(format!(
                "fixture git refused ({}): {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        String::from_utf8(output.stdout)
            .map(|text| text.trim().into())
            .map_err(|error| format!("fixture git output is not UTF-8: {error}"))
    }

    pub(in super::super) fn repository(&self, code: &str) -> std::path::PathBuf {
        let source = self.root.path().join("source");
        let origin = self.root.path().join("origin.git");
        std::fs::create_dir_all(source.join("src")).unwrap();
        for args in [
            vec!["init", "-q", "-b", "main"],
            vec!["config", "user.name", "Fixture"],
            vec!["config", "user.email", "fixture@example.invalid"],
        ] {
            self.git(&source, &args).unwrap();
        }
        plumb::config::detached("git")
            .args(["init", "-q", "--bare"])
            .arg(&origin)
            .status()
            .unwrap();
        self.git(
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
        self.git(&source, &["add", "."]).unwrap();
        self.git(&source, &["commit", "-q", "-m", "base"]).unwrap();
        self.git(&source, &["push", "-q", "origin", "main"])
            .unwrap();
        self.git(&source, &["fetch", "origin"]).unwrap();
        std::fs::write(self.root.path().join("issue.json"), json!({"number":17,"type":{"name":"Auto"},"body":issue::body("Example/probe"),"state":"open","labels":[],"node_id":"fixture-issue","html_url":"https://github.com/Example/probe/issues/17","title":"Follow","updated_at":"2026-10-08T00:00:00Z","user":{"id":1,"login":"fixture-writer[bot]","type":"Bot"}}).to_string()).unwrap();
        super::super::github::executable(&self.command, "recovery");
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
        let cloned = plumb::config::detached("git")
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
    let plan = state.plan.unwrap();
    let current = issue::snapshot(&fixture.provider(), 17).unwrap();
    assert_eq!(plan.issue.updated, current.updated);
    fixture
        .provider()
        .patch(
            "repos/Example/probe/issues/17",
            &json!({"updated_at":"2026-10-09T01:00:00Z"}),
        )
        .unwrap();
    let changed = issue::snapshot(&fixture.provider(), 17).unwrap();
    let request = plumb::delivery::Request {
        root: &state.worktree,
        repository: "Example/probe",
        issue: &changed,
        observed: plan.observed,
        base: &plan.base,
        pull: &plan.pull,
    };
    let authority = plumb::guard::Authority::running().unwrap();
    let refusal = plumb::delivery::revalidate(request, &plan, &authority)
        .err()
        .unwrap();
    assert!(refusal.to_string().contains("Issue snapshot"));
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
