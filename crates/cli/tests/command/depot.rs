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
fn explicit() {
    for kind in ["configuration", "skill", "changelog"] {
        let help = Command::new(env!("CARGO_BIN_EXE_plumb"))
            .args(["depot", kind, "--help"])
            .output()
            .expect("depot help");
        assert!(help.status.success());
        let text = String::from_utf8_lossy(&help.stdout);
        assert!(text.contains("--from <FROM>"), "{kind}: {text}");
        assert!(!text.contains("--keep"), "{kind}: {text}");

        let missing = Command::new(env!("CARGO_BIN_EXE_plumb"))
            .args(["depot", kind, "--marker", "v1.2.3"])
            .output()
            .expect("missing source");
        assert!(!missing.status.success());
        assert!(
            String::from_utf8_lossy(&missing.stderr).contains("--from <FROM>"),
            "{kind}"
        );
    }
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

#[test]
#[cfg(unix)]
fn exact() {
    use std::os::unix::fs::PermissionsExt as _;

    let remote = tempfile::tempdir().expect("remote");
    let home = tempfile::tempdir().expect("home");
    let repo = tempfile::tempdir().expect("repository");
    let tools = tempfile::tempdir().expect("tools");
    let version = format!("v{}", env!("CARGO_PKG_VERSION"));
    let release = plumb::depot::v2::Release {
        product: "plumb".into(),
        channel: "stable".into(),
        version: version.clone(),
        commit: "1".repeat(40),
        seal: plumb::depot::v2::Seal {
            url: format!("https://releases.plumb.perish.uk/v1/releases/stable/{version}/seal.json"),
            sha256: "2".repeat(64),
        },
    };
    let hooks = [
        (
            "assets/git/hooks/pre-commit",
            "#!/bin/sh\nexec plumb guard .\n",
        ),
        (
            "assets/git/hooks/commit-msg",
            "#!/bin/sh\nexec plumb guard . --attach \"$1\"\n",
        ),
    ];
    let manifest = plumb::depot::v2::Manifest {
        format: plumb::depot::v2::FORMAT,
        source: "https://depot.test".into(),
        derivative: plumb::depot::v2::Kind::Configuration,
        release: release.clone(),
        snapshot: plumb::depot::v2::Snapshot {
            timestamp: "20260831T120000Z".into(),
            commit: "3".repeat(40),
        },
        objects: hooks
            .iter()
            .map(|(path, body)| plumb::depot::Object {
                path: (*path).into(),
                sha256: plumb::depot::sha(body.as_bytes()),
                size: body.len() as u64,
            })
            .collect(),
    };
    let body = manifest.encode().expect("manifest");
    let pointer = plumb::depot::v2::Pointer::new(&manifest, body.as_bytes()).expect("pointer");
    let snapshot =
        plumb::depot::v2::snapshots(&release, manifest.derivative, &manifest.snapshot.timestamp)
            .expect("snapshot");
    for (path, text) in hooks {
        let target = remote.path().join(&snapshot).join(path);
        std::fs::create_dir_all(target.parent().expect("object parent")).expect("object root");
        std::fs::write(target, text).expect("object");
    }
    std::fs::write(
        remote.path().join(&snapshot).join(plumb::depot::v2::LEAF),
        body,
    )
    .expect("manifest");
    let exact =
        plumb::depot::v2::exact("plumb", manifest.derivative, "stable", &version).expect("exact");
    let target = remote.path().join(exact);
    std::fs::create_dir_all(target.parent().expect("pointer parent")).expect("pointer root");
    std::fs::write(target, pointer.encode().expect("pointer")).expect("pointer");
    Command::new("git")
        .args(["-C", repo.path().to_str().expect("repo"), "init", "-q"])
        .status()
        .expect("git");
    std::fs::write(repo.path().join("plumb.toml"), "[layout]\n").expect("governance");
    let curl = tools.path().join("curl");
    std::fs::write(
        &curl,
        "#!/bin/sh\nout=\nurl=\nwhile [ \"$#\" -gt 0 ]; do\n  if [ \"$1\" = --output ]; then out=$2; shift 2; else url=$1; shift; fi\ndone\nsource=$FIXTURE_DEPOT/${url#https://depot.test/}\nif [ -f \"$source\" ]; then cp \"$source\" \"$out\"; printf 200; else printf 404; fi\n",
    )
    .expect("curl");
    std::fs::set_permissions(&curl, std::fs::Permissions::from_mode(0o755)).expect("curl mode");
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["depot", "sync", repo.path().to_str().expect("repo")])
        .env("PLUMB_HOME", home.path())
        .env("PLUMB_RULES_SOURCE", "https://depot.test")
        .env("FIXTURE_DEPOT", remote.path())
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
    assert!(String::from_utf8_lossy(&output.stdout).contains("20260831T120000Z"));
    assert_eq!(
        std::fs::read_to_string(repo.path().join(".git/hooks/pre-commit")).expect("hook"),
        "#!/bin/sh\nexec plumb guard .\n"
    );
}
