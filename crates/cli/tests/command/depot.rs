use std::process::Command;

#[path = "../support.rs"]
pub(super) mod support;

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
    let remote = support::Bucket::open(6);
    let authority = format!("{}/workflow", remote.endpoint());
    let source = tempfile::tempdir().expect("source");
    let home = tempfile::tempdir().expect("home");
    let repo = tempfile::tempdir().expect("repository");
    let status = Command::new("git")
        .args(["-C", repo.path().to_str().expect("repo"), "init", "-q"])
        .status()
        .expect("git");
    assert!(status.success());
    let status = Command::new("git")
        .args([
            "-C",
            repo.path().to_str().expect("repo"),
            "remote",
            "add",
            "origin",
            "ssh://git@git.perish.top/PerishFire/probe.git",
        ])
        .status()
        .expect("git");
    assert!(status.success());
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
    let profile = "schema = \"plumb.product-profile/v1\"\n\n[product]\nname = \"probe\"\nauthority = \"https://releases.probe.perish.uk\"\nderivatives = [\"skill\"]\n\n[governance]\nmanifest = \"[layout]\"\nectropy = \"[comment]\\nallow = false\"\n";
    let digest = plumb::depot::sha(profile.as_bytes());
    for (path, body) in [
        (
            "rules/products.toml".to_string(),
            format!(
                "schema = \"plumb.products/v2\"\n\n[[product]]\nidentity = \"git.perish.top/PerishFire/probe\"\nprofile = \"{digest}\"\n"
            ),
        ),
        (format!("profiles/{digest}.toml"), profile.to_string()),
    ] {
        let target = source.path().join(path);
        std::fs::create_dir_all(target.parent().expect("profile parent")).expect("profile root");
        std::fs::write(target, body).expect("profile");
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
    assert!(!repo.path().join("plumb.toml").exists());
    assert!(home.path().join("configurations/latest.json").is_file());
    assert!(!home.path().join("depot").exists());
    remote.finish();
}
