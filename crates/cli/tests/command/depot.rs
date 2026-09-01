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
    let depot = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["depot", "--help"])
        .output()
        .expect("depot help");
    assert!(!String::from_utf8_lossy(&depot.stdout).contains("sync"));
    let install = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["configuration", "install", "--help"])
        .output()
        .expect("configuration help");
    let text = String::from_utf8_lossy(&install.stdout);
    assert!(text.contains("--version <VERSION>"), "{text}");
    assert!(text.contains("--path <PATH>"), "{text}");
    let foreign = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["configuration", "install", "--version", "v99.0.0"])
        .output()
        .expect("foreign configuration");
    assert!(!foreign.status.success());
    assert!(String::from_utf8_lossy(&foreign.stderr).contains("requires an explicit --path"));
}

#[test]
#[cfg(unix)]
fn install() {
    let remote = support::Bucket::open(4);
    let authority = format!("{}/workflow", remote.endpoint());
    let source = tempfile::tempdir().expect("source");
    let home = tempfile::tempdir().expect("home");
    let repo = tempfile::tempdir().expect("repository");
    let status = Command::new("git")
        .args(["-C", repo.path().to_str().expect("repo"), "init", "-q"])
        .status()
        .expect("git");
    assert!(status.success());
    std::fs::write(repo.path().join("plumb.toml"), "[layout]\n").expect("governance");
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
    for (path, body) in hooks {
        let target = source.path().join(path);
        std::fs::create_dir_all(target.parent().expect("hook parent")).expect("hook root");
        std::fs::write(target, body).expect("hook");
    }
    let version = format!("v{}", env!("CARGO_PKG_VERSION"));
    let bundle = plumb::depot::v3::Bundle::read(
        source.path(),
        plumb::depot::v3::Identity {
            product: "plumb".into(),
            channel: "stable".into(),
            version: version.clone(),
            marker: plumb::depot::v3::Marker {
                name: version.clone(),
                sha256: "a".repeat(64),
            },
            kind: plumb::depot::v3::Kind::Configuration,
        },
    )
    .expect("bundle");
    let pointer = plumb::depot::v3::Pointer::new(
        &bundle.manifest,
        plumb::depot::v3::Publication {
            source: &authority,
            prior: None,
            created: "2026-09-01T01:02:03Z".into(),
        },
    )
    .expect("pointer");
    let route =
        plumb::depot::v3::Route::new("stable", plumb::depot::v3::Kind::Configuration, &version);
    let generation = plumb::depot::v3::generation(route, &pointer.generation).expect("route");
    for (path, body) in &bundle.bodies {
        remote.seed(&format!("{generation}/objects/{path}"), body);
    }
    remote.seed(
        &format!("{generation}/{}", plumb::depot::v3::LEAF),
        &bundle.manifest.encode().expect("manifest"),
    );
    let latest = plumb::depot::v3::latest(route).expect("latest");
    remote.seed(&latest, &pointer.encode().expect("pointer"));
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args([
            "configuration",
            "install",
            repo.path().to_str().expect("repo"),
        ])
        .env("PLUMB_HOME", home.path())
        .env("PLUMB_RULES_SOURCE", &authority)
        .output()
        .expect("install");
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
    assert!(home.path().join("configurations/latest.json").is_file());
    assert!(!home.path().join("depot").exists());
    remote.finish();
}
