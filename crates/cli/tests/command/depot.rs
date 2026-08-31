use std::path::Path;
use std::process::Command;

#[path = "../support.rs"]
mod support;

fn repository(name: &str) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("repository fixture");
    let remote = format!("ssh://git@git.perish.top/PerishFire/{name}.git");
    for args in [vec!["init", "-q"], vec!["remote", "add", "origin", &remote]] {
        let done = Command::new("git")
            .arg("-C")
            .arg(root.path())
            .args(args)
            .status()
            .expect("git should run");
        assert!(done.success(), "fixture should become a repository");
    }
    root
}

fn publish(root: &Path, home: &Path) -> (bool, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args([
            "depot",
            "skill",
            root.to_str().expect("repository path should be utf8"),
            "--marker",
            "invalid",
            "--from",
            root.to_str().expect("source path should be utf8"),
            "--dry-run",
        ])
        .env("PLUMB_HOME", home)
        .output()
        .expect("plumb should run");
    let mut held = String::from_utf8_lossy(&output.stdout).to_string();
    held.push_str(&String::from_utf8_lossy(&output.stderr));
    (output.status.success(), held)
}

#[test]
fn mapped() {
    let depot = support::depot(&[]);
    let ectropy = repository("ectropy");
    let (ok, held) = publish(ectropy.path(), depot.path());
    assert!(!ok, "{held}");
    assert!(held.contains("invalid release marker vinvalid"), "{held}");
    assert!(!held.contains("plumb.toml"), "{held}");

    let unknown = repository("unknown");
    let (ok, held) = publish(unknown.path(), depot.path());
    assert!(!ok, "{held}");
    assert!(
        held.contains(
            "perish.code product identity git.perish.top/PerishFire/unknown is absent from the Plumb depot"
        ),
        "{held}"
    );
}

#[test]
fn refused() {
    let rules = r#"
schema = "plumb.products/v1"

[[product]]
identity = "git.perish.top/PerishFire/ectropy"
name = "ectropy"
authority = "https://releases.ectropy.perish.uk"
derivatives = ["changelog"]
"#;
    let depot = support::depot(&[("rules/products.toml", rules)]);
    let ectropy = repository("ectropy");
    let (ok, held) = publish(ectropy.path(), depot.path());
    assert!(!ok, "{held}");
    assert!(
        held.contains("perish.code product ectropy does not carry the skill derivative"),
        "{held}"
    );
}

#[test]
#[cfg(unix)]
fn sync() {
    use std::os::unix::fs::PermissionsExt as _;

    let source = support::depot(&[]);
    let home = tempfile::tempdir().expect("home");
    let repo = tempfile::tempdir().expect("repository");
    let tools = tempfile::tempdir().expect("tools");
    let status = Command::new("git")
        .args(["-C", repo.path().to_str().expect("repo"), "init", "-q"])
        .status()
        .expect("git");
    assert!(status.success());
    std::fs::write(repo.path().join("plumb.toml"), "[layout]\n").expect("governance");
    std::fs::write(repo.path().join(".git/hooks/pre-commit"), "foreign\n").expect("foreign hook");
    let curl = tools.path().join("curl");
    std::fs::write(
        &curl,
        "#!/bin/sh\nout=\nurl=\nwhile [ \"$#\" -gt 0 ]; do\n  if [ \"$1\" = --output ]; then out=$2; shift 2; else url=$1; shift; fi\ndone\nkey=${url#https://depot.test/}\ncase $key in\n  channels/stable/latest/metadata.json) source=$FIXTURE_DEPOT/metadata.json ;;\n  channels/stable/versions/*) source=$FIXTURE_DEPOT/${key#channels/stable/versions/} ;;\n  *) source= ;;\nesac\nif [ -n \"$source\" ] && [ -f \"$source\" ]; then cp \"$source\" \"$out\"; printf 200; else printf 404; fi\n",
    )
    .expect("curl");
    std::fs::set_permissions(&curl, std::fs::Permissions::from_mode(0o755)).expect("curl mode");
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["depot", "sync", repo.path().to_str().expect("repo")])
        .env("PLUMB_HOME", home.path())
        .env("PLUMB_RULES_SOURCE", "https://depot.test")
        .env("FIXTURE_DEPOT", source.path().join("depot"))
        .env(
            "PATH",
            format!(
                "{}:{}",
                tools.path().display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .output()
        .expect("sync");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(repo.path().join(".git/hooks/pre-commit")).expect("hook"),
        "#!/bin/sh\nexec plumb guard .\n"
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("projected Plumb guard hooks"));
}
