use super::{Repo, cache};
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
fn selection() {
    for options in [&["-a"][..], &["--", "src/lib.rs"]] {
        let (fixture, home) = super::hook::installed(None);
        let root = fixture.path();
        Repo::git(root, &["checkout", "-qb", "topic"]);
        Repo::git(
            root,
            &[
                "remote",
                "add",
                "origin",
                "https://example.invalid/Example/probe",
            ],
        );
        std::fs::write(root.join("src/lib.rs"), "pub fn answer() -> u8 { 43 }\n").expect("change");
        std::fs::remove_file(root.join("tests/probe.rs")).expect("delete");
        if options[0] == "--" {
            Repo::git(root, &["add", "-u", "--", "tests/probe.rs"]);
        }
        let commit = super::hook::hooked(root, home.path())
            .args(["commit", "-qm", "candidate"])
            .args(options)
            .output()
            .expect("commit");
        super::cache::success(&commit);
        plumb::guard::commit(root, "HEAD").expect("committed proof");
        let diff = Repo::git(
            root,
            &["diff-tree", "--no-commit-id", "--name-status", "-r", "HEAD"],
        );
        let expected = if options[0] == "-a" {
            "M\tsrc/lib.rs\nD\ttests/probe.rs\n"
        } else {
            "M\tsrc/lib.rs\n"
        };
        assert_eq!(diff.stdout, expected.as_bytes());
        if options[0] == "--" {
            let staged = Repo::git(root, &["diff", "--cached", "--name-status"]);
            assert_eq!(staged.stdout, b"D\ttests/probe.rs\n");
        }
    }
}

#[test]
fn mismatch() {
    let (fixture, home) = super::hook::installed(None);
    let root = fixture.path();
    Repo::git(root, &["checkout", "-qb", "topic"]);
    Repo::git(
        root,
        &[
            "remote",
            "add",
            "origin",
            "https://example.invalid/Example/probe",
        ],
    );
    let head = Repo::git(root, &["rev-parse", "HEAD"]);
    let foreign = tempfile::tempdir().expect("foreign index directory");
    let index = foreign.path().join("index");
    std::fs::write(&index, "not this repository's index").expect("foreign index");
    let preflight = super::hook::hooked(root, home.path())
        .args(["hook", "run", "pre-commit"])
        .env("GIT_INDEX_FILE", &index)
        .output()
        .expect("preflight");
    super::cache::success(&preflight);

    let hooks = tempfile::tempdir().expect("message hook directory");
    std::fs::copy(
        root.join(".git/hooks/commit-msg"),
        hooks.path().join("commit-msg"),
    )
    .expect("copy installed message hook");
    std::fs::write(root.join("src/lib.rs"), "pub fn answer() -> u8 { 43 }\n").expect("change");
    let refused = super::hook::hooked(root, home.path())
        .arg("-c")
        .arg(format!("core.hooksPath={}", hooks.path().display()))
        .args(["commit", "-aqm", "mismatched proof"])
        .output()
        .expect("commit");
    assert!(!refused.status.success());
    let error = String::from_utf8_lossy(&refused.stderr);
    assert!(
        error.contains("no matching pre-commit guard proof for commit tree"),
        "{error}"
    );
    assert_eq!(Repo::git(root, &["rev-parse", "HEAD"]).stdout, head.stdout);
}
