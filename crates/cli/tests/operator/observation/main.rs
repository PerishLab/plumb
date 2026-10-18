#![cfg(unix)]

#[path = "../../support/mod.rs"]
mod support;
mod world;
use world::Repo;

#[test]
fn currency() {
    let fixture = world::fixture();
    let root = fixture.path();
    let home = support::home();
    let tools = tempfile::tempdir().unwrap();
    std::fs::write(
        root.join("plumb.toml"),
        "[workflow.hash.guard]\nplumb=['Cargo.toml','Cargo.lock','src','pnpm-lock.yaml']\n",
    )
    .unwrap();
    std::fs::write(
        root.join("pnpm-lock.yaml"),
        "lockfileVersion: '9.0'\npackages:\n  '@perishlab/probe@0.1.0': {}\n",
    )
    .unwrap();
    Repo::git(root, &["add", "."]);
    let latest = tools.path().join("latest");
    let calls = tools.path().join("calls");
    std::fs::write(&latest, "\"0.2.0\"").unwrap();
    support::executable(
        &tools.path().join("pnpm"),
        r#"#!/bin/sh
seat=$(dirname "$0")
if [ "$1" != --config.ignore-pnpmfile=true ]; then exit 91; fi
shift
if [ "$1" = view ]; then cat "$seat/latest"; else exit 92; fi
"#,
    );
    support::executable(
        &tools.path().join("plumb"),
        r#"#!/bin/sh
seat=$(dirname "$0")
if [ "$1" = --version ]; then echo fixture; exit 0; fi
if [ "$1" != doctor ] || [ -n "${WHARF_PACKAGES_TOKEN:-}" ] || [ "$NPM_CONFIG_USERCONFIG" != /dev/null ]; then exit 93; fi
printf '%s\n' "$PLUMB_PACKAGES_OBSERVATION" >> "$seat/calls"
"#,
    );
    let mut paths = vec![tools.path().to_path_buf()];
    paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    let path = std::env::join_paths(paths).unwrap();
    let run = || {
        support::plumb()
            .args(["guard", ".", "--json"])
            .current_dir(root)
            .env("PLUMB_HOME", home.path())
            .env("PATH", &path)
            .env("WHARF_PACKAGES_TOKEN", "SYNTHETIC_ONLY_ENV")
            .output()
            .unwrap()
    };
    let first = run();
    world::success(&first);
    let observed = std::fs::read_to_string(&calls).unwrap();
    assert!(observed.contains("0.2.0"));
    let cached = run();
    world::success(&cached);
    assert_eq!(first.stdout, cached.stdout);
    assert_eq!(std::fs::read_to_string(&calls).unwrap(), observed);
    std::fs::write(&latest, "\"0.3.0\"").unwrap();
    let changed = run();
    world::success(&changed);
    assert_ne!(first.stdout, changed.stdout);
    let recorded = std::fs::read_to_string(&calls).unwrap();
    assert_eq!(recorded.lines().count(), 2);
    assert!(recorded.contains("0.3.0"));
    std::fs::write(&latest, "\"0.4.0-rc.1\"").unwrap();
    let unknown = run();
    assert!(!unknown.status.success());
    assert_eq!(std::fs::read_to_string(&calls).unwrap(), recorded);
    assert!(String::from_utf8_lossy(&unknown.stderr).contains("prerelease"));
}

#[test]
fn doctor() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path();
    Repo::git(root, &["init", "-q", "-b", "topic"]);
    Repo::git(root, &["config", "user.name", "Plumb Test"]);
    Repo::git(root, &["config", "user.email", "plumb@example.invalid"]);
    Repo::git(
        root,
        &[
            "remote",
            "add",
            "origin",
            "https://git.example.invalid/Example/probe.git",
        ],
    );
    let home = support::home();
    let tools = tempfile::tempdir().unwrap();
    std::fs::write(root.join("plumb.toml"), "[workflow.hash.guard]\nplumb=['package.json','pnpm-lock.yaml']\n[[layout.file]]\nname=['AGENTS.md']\nkind='retired'\n[[layout.file]]\nname=['package.json','pnpm-lock.yaml']\n").unwrap();
    support::hooks(root);
    std::fs::write(
        root.join("package.json"),
        r#"{"dependencies":{"@perishlab/probe":"0"}}"#,
    )
    .unwrap();
    std::fs::write(root.join("pnpm-lock.yaml"), "lockfileVersion: '9.0'\nimporters:\n  .:\n    dependencies:\n      '@perishlab/probe':\n        specifier: '0'\n        version: 0.1.0\npackages:\n  '@perishlab/probe@0.1.0': {}\n").unwrap();
    Repo::git(root, &["add", "."]);
    let latest = tools.path().join("latest");
    let calls = tools.path().join("calls");
    std::fs::write(&latest, "\"0.1.0\"").unwrap();
    let version = std::process::Command::new("pnpm")
        .arg("--version")
        .output()
        .unwrap();
    assert!(version.status.success());
    std::fs::write(tools.path().join("version"), version.stdout).unwrap();
    support::executable(
        &tools.path().join("pnpm"),
        r#"#!/bin/sh
seat=$(dirname "$0")
if [ "$1" = --version ]; then cat "$seat/version"; exit 0; fi
if [ "$1" != --config.ignore-pnpmfile=true ]; then exit 91; fi
shift
if [ "$1" = view ]; then cat "$seat/latest"; else exit 92; fi
"#,
    );
    let candidate = format!("'{}'", env!("CARGO_BIN_EXE_plumb").replace('\'', "'\\''"));
    support::executable(
        &tools.path().join("plumb"),
        &format!(
            r#"#!/bin/sh
seat=$(dirname "$0")
if [ "$1" = --version ]; then exec {candidate} "$@"; fi
if [ "$1" != doctor ] || [ -n "${{WHARF_PACKAGES_TOKEN:-}}" ] || [ "$NPM_CONFIG_USERCONFIG" != /dev/null ]; then exit 93; fi
printf '%s\n' "$PLUMB_PACKAGES_OBSERVATION" >> "$seat/calls"
{candidate} "$@" > "$seat/said"
status=$?
cat "$seat/said"
exit $status
"#
        ),
    );
    let mut paths = vec![tools.path().to_path_buf()];
    paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    let path = std::env::join_paths(paths).unwrap();
    let run = || {
        support::plumb()
            .args(["guard", ".", "--json"])
            .current_dir(root)
            .env("PLUMB_HOME", home.path())
            .env("PATH", &path)
            .env("WHARF_PACKAGES_TOKEN", "SYNTHETIC_ONLY_ENV")
            .output()
            .unwrap()
    };
    let first = run();
    world::success(&first);
    let recorded = std::fs::read_to_string(&calls).unwrap();
    assert_eq!(recorded.lines().count(), 1);
    let cached = run();
    world::success(&cached);
    assert_eq!(first.stdout, cached.stdout);
    assert_eq!(std::fs::read_to_string(&calls).unwrap(), recorded);
    std::fs::write(&latest, "\"0.2.0\"").unwrap();
    let message = tools.path().join("message");
    std::fs::write(&message, "Lagging lock\n").unwrap();
    let attached = support::plumb()
        .args(["guard", ".", "--attach"])
        .arg(&message)
        .current_dir(root)
        .env("PLUMB_HOME", home.path())
        .env("PATH", &path)
        .env("WHARF_PACKAGES_TOKEN", "SYNTHETIC_ONLY_ENV")
        .output()
        .unwrap();
    world::success(&attached);
    assert_eq!(std::fs::read_to_string(&calls).unwrap().lines().count(), 2);
    let said = std::fs::read_to_string(tools.path().join("said")).unwrap();
    assert!(
        said.contains("noted: npm @perishlab/probe is locked at 0.1.0, stable latest is 0.2.0; the organization Guard late-binds it")
            && !said.contains("out of true:"),
        "{said}"
    );
    let stale = run();
    world::success(&stale);
    assert!(String::from_utf8_lossy(&stale.stdout).contains("local-locked"));
    assert_eq!(std::fs::read_to_string(&calls).unwrap().lines().count(), 2);
}
