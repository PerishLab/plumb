use super::precommit::{Repo, cache};
use super::support;
use serde_json::Value;
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Output};

struct Fixture {
    root: tempfile::TempDir,
    home: tempfile::TempDir,
    tools: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let root = cache::fixture();
        std::fs::write(root.path().join("package.json"),
            "{\"name\":\"fixture\",\"dependencies\":{\"@perishlab/probe\":\"0.1.0\",\"other\":\"1\"}}\n").unwrap();
        std::fs::write(root.path().join("pnpm-lock.yaml"), Self::lock("0.1.0")).unwrap();
        Repo::git(root.path(), &["add", "."]);
        Repo::git(root.path(), &["commit", "-q", "-m", "fixture"]);
        let tools = tempfile::tempdir().unwrap();
        std::fs::write(tools.path().join("latest"), "\"0.2.0\"\n").unwrap();
        std::fs::write(tools.path().join("lock"), Self::lock("0.2.0")).unwrap();
        let pnpm = tools.path().join("pnpm");
        std::fs::write(
            &pnpm,
            r#"#!/bin/sh
seat=$(dirname "$0")
case "$1" in
view) cat "$seat/latest" ;;
update) cp "$seat/lock" pnpm-lock.yaml ;;
--version) echo fixture ;;
*) exit 91 ;;
esac
"#,
        )
        .unwrap();
        std::fs::set_permissions(pnpm, std::fs::Permissions::from_mode(0o755)).unwrap();
        Self {
            root,
            home: support::home(),
            tools,
        }
    }

    fn lock(version: &str) -> String {
        format!("lockfileVersion: '9.0'\npackages:\n  '@perishlab/probe@{version}': {{}}\n")
    }

    fn command(&self, verb: &str) -> Command {
        let mut paths = vec![self.tools.path().to_path_buf()];
        paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
        let mut command = support::plumb();
        command
            .args([verb, ".", "--json"])
            .current_dir(self.root.path())
            .env("PLUMB_HOME", self.home.path())
            .env("PATH", std::env::join_paths(paths).unwrap());
        command
    }

    fn git(&self, args: &[&str]) -> String {
        String::from_utf8(Repo::git(self.root.path(), args).stdout)
            .unwrap()
            .trim()
            .into()
    }

    fn report(output: &Output) -> Value {
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }
}

#[test]
fn lift() {
    let fixture = Fixture::new();
    let head = fixture.git(&["rev-parse", "HEAD"]);
    let tree = fixture.git(&["write-tree"]);
    let output = fixture.command("lift").output().unwrap();
    let resolution = Fixture::report(&output);
    assert_eq!(resolution["context"], "lift");
    assert_eq!(resolution["packages"][0]["version"], "0.2.0");
    assert_ne!(resolution["tree"], tree);
    assert_eq!(fixture.git(&["rev-parse", "HEAD"]), head);
    assert_eq!(
        fixture.git(&["write-tree"]),
        tree,
        "lift leaves its delta unstaged"
    );
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(fixture.root.path().join("package.json")).unwrap())
            .unwrap();
    assert_eq!(manifest["dependencies"]["@perishlab/probe"], "0");
    assert_eq!(manifest["dependencies"]["other"], "1");
    assert!(
        std::fs::read_to_string(fixture.root.path().join("pnpm-lock.yaml"))
            .unwrap()
            .contains("0.2.0")
    );
}

#[test]
fn runtime() {
    let fixture = Fixture::new();
    let head = fixture.git(&["rev-parse", "HEAD"]);
    let tree = fixture.git(&["write-tree"]);
    let output = fixture
        .command("guard")
        .env("PLUMB_GUARD_STRENGTH", "full")
        .env("PLUMB_GUARD_BOUNDARY", "head")
        .output()
        .unwrap();
    let evidence = Fixture::report(&output);
    assert_eq!(evidence["guard"]["resolution"]["context"], "ci-latest");
    assert_eq!(
        evidence["guard"]["resolution"]["packages"][0]["version"],
        "0.2.0"
    );
    assert_eq!(evidence["guard"]["tree"], tree);
    assert_ne!(evidence["guard"]["resolution"]["tree"], tree);
    assert_eq!(fixture.git(&["rev-parse", "HEAD"]), head);
    assert_eq!(fixture.git(&["status", "--porcelain"]), "");
    assert!(
        std::fs::read_to_string(fixture.root.path().join("pnpm-lock.yaml"))
            .unwrap()
            .contains("0.1.0")
    );
}

#[test]
fn local() {
    let fixture = Fixture::new();
    std::fs::write(fixture.tools.path().join("latest"), "registry unreadable\n").unwrap();
    let output = fixture.command("guard").output().unwrap();
    let proof: plumb::guard::Descriptor = serde_json::from_value(Fixture::report(&output)).unwrap();
    proof.validate().unwrap();
    let resolution = proof.resolution.unwrap();
    assert_eq!(resolution.context, "local-locked");
    assert_eq!(resolution.packages[0].version, "0.1.0");
}

#[test]
fn refusals() {
    let fixture = Fixture::new();
    std::fs::write(fixture.tools.path().join("latest"), "registry unreadable\n").unwrap();
    let before = fixture.git(&["write-tree"]);
    let failed = fixture.command("lift").output().unwrap();
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("cannot read npm stable"));
    assert_eq!(fixture.git(&["status", "--porcelain"]), "");
    assert_eq!(fixture.git(&["write-tree"]), before);
    Repo::git(fixture.root.path(), &["branch", "-m", "main"]);
    let main = fixture.command("lift").output().unwrap();
    assert!(!main.status.success());
    assert!(String::from_utf8_lossy(&main.stderr).contains("topic worktree"));
}

#[test]
fn graph() {
    let fixture = Fixture::new();
    let first = format!(
        "{}  '@perishlab/transitive@0.1.0': {{}}\n",
        Fixture::lock("0.2.0")
    );
    let settled = format!(
        "{}  '@perishlab/transitive@0.2.0': {{}}\n",
        Fixture::lock("0.2.0")
    );
    std::fs::write(fixture.tools.path().join("first"), first).unwrap();
    std::fs::write(fixture.tools.path().join("lock"), settled).unwrap();
    std::fs::write(
        fixture.tools.path().join("pnpm"),
        r#"#!/bin/sh
seat=$(dirname "$0")
case "$1" in
view) cat "$seat/latest" ;;
update)
if grep -q transitive pnpm-lock.yaml; then cp "$seat/lock" pnpm-lock.yaml
else cp "$seat/first" pnpm-lock.yaml; fi ;;
*) exit 91 ;;
esac
"#,
    )
    .unwrap();
    let output = fixture.command("lift").output().unwrap();
    let result = Fixture::report(&output);
    assert_eq!(result["packages"].as_array().unwrap().len(), 2);
    assert!(
        result["packages"]
            .as_array()
            .unwrap()
            .iter()
            .all(|package| package["version"] == "0.2.0")
    );
}

#[test]
fn missing() {
    let fixture = Fixture::new();
    std::fs::write(
        fixture.tools.path().join("lock"),
        "lockfileVersion: '9.0'\npackages: {}\n",
    )
    .unwrap();
    let output = fixture.command("lift").output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing from its lock"));
    assert_eq!(fixture.git(&["status", "--porcelain"]), "");
}

#[test]
fn current() {
    let fixture = Fixture::new();
    Fixture::report(&fixture.command("lift").output().unwrap());
    Repo::git(fixture.root.path(), &["add", "."]);
    Repo::git(
        fixture.root.path(),
        &["commit", "-q", "-m", "current fixture"],
    );
    let before = fixture.git(&["rev-parse", "HEAD"]);
    let result = Fixture::report(&fixture.command("lift").output().unwrap());
    assert_eq!(result["packages"][0]["version"], "0.2.0");
    assert_eq!(fixture.git(&["status", "--porcelain"]), "");
    assert_eq!(fixture.git(&["rev-parse", "HEAD"]), before);
    assert_eq!(result["tree"], fixture.git(&["write-tree"]));
}
