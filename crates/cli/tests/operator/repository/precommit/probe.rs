use super::{Repo, cache, support};
use std::path::Path;

fn rules() -> String {
    let output = std::process::Command::new("git")
        .arg("--version")
        .output()
        .unwrap();
    assert!(output.status.success());
    let version = String::from_utf8(output.stdout).unwrap();
    format!(
        "[member]\n[[member.entry]]\nname='good'\n[[member.entry.probe]]\nargv=['git','--version']\nstdout={version:?}\n[[member.entry]]\nname='bad'\n[[member.entry.probe]]\nargv=['plumb-probe-never-start']\nstdout=''\n"
    )
}

fn manifest(root: &Path, rule: &str) {
    std::fs::write(root.join("plumb.toml"), format!(
        "[workflow.hash.guard]\nrust=['Cargo.toml','Cargo.lock','src']\n[[layout.file]]\nname=['Cargo.toml']\nrule=['rule://seat/{rule}']\n[[layout.file]]\nname=['package.json']\nrule=['rule://seat/bad']\n"
    )).unwrap();
}

#[test]
fn staged() {
    let fixture = cache::fixture();
    let root = fixture.path();
    manifest(root, "good");
    Repo::git(root, &["add", "plumb.toml"]);
    let home = support::overlay(&[("rules/seat.toml", &rules())]);
    let first = cache::run(root, home.path());
    cache::success(&first);
    manifest(root, "bad");
    let unstaged = cache::run(root, home.path());
    cache::success(&unstaged);
    assert_eq!(
        first.stdout, unstaged.stdout,
        "working rules cannot change the staged proof"
    );
    Repo::git(root, &["add", "plumb.toml"]);
    let refused = cache::run(root, home.path());
    assert!(!refused.status.success());
    let error = String::from_utf8_lossy(&refused.stderr);
    assert!(
        error.contains("cannot resolve tool plumb-probe-never-start"),
        "{error}"
    );
    assert!(
        !error.contains("guard guard/rust"),
        "cached proof must not bypass rule preflight"
    );
}

#[test]
fn scope() {
    let fixture = cache::fixture();
    let root = fixture.path();
    manifest(root, "good");
    Repo::git(root, &["add", "plumb.toml"]);
    let home = support::overlay(&[("rules/seat.toml", &rules())]);
    let first = cache::run(root, home.path());
    cache::success(&first);
    std::fs::write(root.join("package.json"), "{}\n").unwrap();
    Repo::git(root, &["add", "package.json"]);
    let next = cache::run(root, home.path());
    cache::success(&next);
    let first: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    let next: serde_json::Value = serde_json::from_slice(&next.stdout).unwrap();
    assert_eq!(
        first["actions"], next["actions"],
        "unrelated file probes do not enter the Rust world"
    );
}

#[test]
#[cfg(unix)]
fn once() {
    use std::os::unix::fs::PermissionsExt as _;
    let fixture = cache::fixture();
    let root = fixture.path();
    manifest(root, "good");
    Repo::git(root, &["add", "plumb.toml"]);
    let output = std::process::Command::new("cargo")
        .arg("--version")
        .output()
        .unwrap();
    assert!(output.status.success());
    let version = String::from_utf8(output.stdout).unwrap();
    let rules = format!(
        "[member]\n[[member.entry]]\nname='good'\n[[member.entry.probe]]\nargv=['cargo','--version']\nstdout={version:?}\n"
    );
    let home = support::overlay(&[("rules/seat.toml", &rules)]);
    let tools = tempfile::tempdir().unwrap();
    let counter = tools.path().join("counter");
    let cargo = tools.path().join("cargo");
    let backend = std::process::Command::new("rustup")
        .args(["which", "cargo"])
        .output()
        .unwrap();
    assert!(backend.status.success());
    let backend = String::from_utf8(backend.stdout).unwrap();
    std::fs::write(&cargo, format!("#!/bin/sh\nif [ \"$1\" = --version ]; then printf 'probe\\n' >> '{}'; fi\nexec '{}' \"$@\"\n", counter.display(), backend.trim())).unwrap();
    std::fs::set_permissions(&cargo, std::fs::Permissions::from_mode(0o755)).unwrap();
    let mut paths = vec![tools.path().to_path_buf()];
    paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    let path = std::env::join_paths(paths).unwrap();
    let run = || {
        std::process::Command::new(env!("CARGO_BIN_EXE_plumb"))
            .args(["guard", ".", "--json"])
            .current_dir(root)
            .env("PLUMB_HOME", home.path())
            .env("PATH", &path)
            .output()
            .unwrap()
    };
    cache::success(&run());
    let before = std::fs::read_to_string(&counter).unwrap().lines().count();
    let output = run();
    cache::success(&output);
    assert!(!String::from_utf8_lossy(&output.stderr).contains("guard guard/rust"));
    let after = std::fs::read_to_string(&counter).unwrap().lines().count();
    assert_eq!(
        after - before,
        1,
        "the matching DSL version probe replaces the legacy version probe"
    );
}
