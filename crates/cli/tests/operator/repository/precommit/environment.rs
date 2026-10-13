use super::{Repo, cache, support};
use std::path::Path;
use std::process::Output;

fn run(root: &Path, home: &Path, input: (&str, &str)) -> Output {
    super::support::plumb()
        .args(["guard", ".", "--json"])
        .current_dir(root)
        .env("PLUMB_HOME", home)
        .env(input.0, input.1)
        .output()
        .expect("guard")
}

#[test]
fn refused() {
    let fixture = cache::fixture();
    let root = fixture.path();
    let home = super::support::home();
    cache::success(&cache::run(root, home.path()));
    for changed in [false, true] {
        if changed {
            std::fs::write(root.join("NOTES"), "unrelated\n").expect("unrelated");
            Repo::git(root, &["add", "NOTES"]);
        }
        for key in [
            "RUSTFLAGS",
            "CARGO_ENCODED_RUSTFLAGS",
            "CARGO_BUILD_TARGET",
            "CC",
        ] {
            let output = run(root, home.path(), (key, "private-invalid-override"));
            assert!(!output.status.success(), "{key} cannot reuse old proof");
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(error.contains(&format!("refuses ambient {key}")), "{error}");
            assert!(!error.contains("private-invalid-override"));
            assert!(!error.contains("guard guard/rust"));
        }
    }
}

#[test]
fn bound() {
    let fixture = cache::fixture();
    let root = fixture.path();
    let home = super::support::home();
    let first = run(root, home.path(), ("TZ", "UTC"));
    cache::success(&first);
    let changed = run(root, home.path(), ("TZ", "Etc/GMT+1"));
    cache::success(&changed);
    assert!(String::from_utf8_lossy(&changed.stderr).contains("guard guard/rust"));
    assert_ne!(first.stdout, changed.stdout);
    let again = run(root, home.path(), ("TZ", "Etc/GMT+1"));
    cache::success(&again);
    assert_eq!(changed.stdout, again.stdout);
    assert!(!String::from_utf8_lossy(&again.stderr).contains("guard guard/rust"));
}

#[test]
fn isolated() {
    let fixture = cache::fixture();
    let root = fixture.path();
    let home = super::support::home();
    std::fs::write(
        root.join("build.rs"),
        "fn main() {\n    assert!(std::env::var_os(\"PLUMB_PROBE_SECRET\").is_none());\n}\n",
    )
    .expect("build probe");
    Repo::git(root, &["add", "build.rs"]);
    let first = run(
        root,
        home.path(),
        ("PLUMB_PROBE_SECRET", "first-private-value"),
    );
    cache::success(&first);
    let second = run(
        root,
        home.path(),
        ("PLUMB_PROBE_SECRET", "second-private-value"),
    );
    cache::success(&second);
    assert_eq!(first.stdout, second.stdout);
    assert!(!String::from_utf8_lossy(&second.stderr).contains("guard guard/rust"));
}

#[test]
fn external() {
    let fixture = cache::fixture();
    let root = fixture.path();
    let home = super::support::home();
    let cargo = tempfile::tempdir().expect("cargo home");
    let path = cargo.path().to_str().expect("path");
    cache::success(&run(root, home.path(), ("CARGO_HOME", path)));
    std::fs::write(
        cargo.path().join("config.toml"),
        "[build]\nrustflags = ['--invalid-probe']\n",
    )
    .expect("host config");
    let output = run(root, home.path(), ("CARGO_HOME", path));
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unbound host configuration"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("guard guard/rust"));
    std::fs::remove_file(cargo.path().join("config.toml")).expect("remove fixture config");
    let parent = home.path().join("tmp/.cargo");
    std::fs::create_dir_all(&parent).expect("managed ancestor");
    std::fs::write(
        parent.join("config"),
        "[build]\nrustflags = ['--invalid-probe']\n",
    )
    .expect("ancestor config");
    let output = run(root, home.path(), ("CARGO_HOME", path));
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unbound host configuration"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("guard guard/rust"));
}

#[test]
fn node() {
    let contract = plumb::config::contract("pnpm").unwrap();
    let clean = contract.capture([]).unwrap();
    let configured = contract
        .capture([("NPM_CONFIG_USERCONFIG".into(), "trusted/npmrc".into())])
        .unwrap();
    assert_eq!(
        configured.get("NPM_CONFIG_USERCONFIG"),
        Some("trusted/npmrc")
    );
    let metadata = contract
        .capture([("NODE_VERSION".into(), "24.18.0".into())])
        .unwrap();
    assert_eq!(
        serde_json::to_string(&clean).unwrap(),
        serde_json::to_string(&metadata).unwrap()
    );
    for key in [
        "NODE_OPTIONS",
        "NODE_PATH",
        "NODE_ENV",
        "NODE_TLS_REJECT_UNAUTHORIZED",
    ] {
        let refused = contract
            .capture([(key.into(), "private-override".into())])
            .err()
            .unwrap();
        assert!(refused.contains(key));
        assert!(!refused.contains("private-override"));
    }
}

#[test]
fn currency() {
    let fixture = cache::fixture();
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
    cache::success(&first);
    let observed = std::fs::read_to_string(&calls).unwrap();
    assert!(observed.contains("0.2.0"));
    let cached = run();
    cache::success(&cached);
    assert_eq!(first.stdout, cached.stdout);
    assert_eq!(std::fs::read_to_string(&calls).unwrap(), observed);
    std::fs::write(&latest, "\"0.3.0\"").unwrap();
    let changed = run();
    cache::success(&changed);
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
