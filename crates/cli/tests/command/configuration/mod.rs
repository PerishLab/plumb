use std::process::Command;

mod depot;
mod fixture;

#[path = "../../support/mod.rs"]
pub(super) mod support;

#[test]
#[cfg(unix)]
fn install() {
    let home = support::home();
    let repo = tempfile::tempdir().expect("repository");
    let status = Command::new("git")
        .args(["-C", repo.path().to_str().expect("repo"), "init", "-q"])
        .status()
        .expect("git");
    assert!(status.success());
    let install = || {
        Command::new(env!("CARGO_BIN_EXE_plumb"))
            .args([
                "configuration",
                "install",
                repo.path().to_str().expect("repo"),
            ])
            .env("PLUMB_HOME", home.path())
            .output()
            .expect("install")
    };
    let bare = install();
    assert!(bare.status.success());
    assert!(String::from_utf8_lossy(&bare.stdout).contains("no hooks projected"));
    assert!(!repo.path().join(".git/hooks/pre-commit").exists());

    std::fs::write(repo.path().join("plumb.toml"), "[layout]\n").expect("manifest");
    let output = install();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for (name, carried) in [
        ("pre-commit", "#!/bin/sh\nexec plumb guard .\n"),
        (
            "commit-msg",
            "#!/bin/sh\nexec plumb guard . --attach \"$1\"\n",
        ),
    ] {
        let hook = repo.path().join(".git/hooks").join(name);
        assert_eq!(std::fs::read_to_string(&hook).expect("hook"), carried);
        use std::os::unix::fs::PermissionsExt as _;
        let mode = std::fs::metadata(&hook).expect("hook").permissions().mode();
        assert_eq!(mode & 0o111, 0o111, "{name} is executable");
    }
    assert!(String::from_utf8_lossy(&output.stdout).contains("projected Plumb guard hooks"));
    assert!(!home.path().join("configurations").exists());
}

#[test]
fn precache() {
    let repo = tempfile::tempdir().expect("repository");
    let status = Command::new("git")
        .args(["-C", repo.path().to_str().expect("repo"), "init", "-q"])
        .status()
        .expect("git");
    assert!(status.success());

    let home = tempfile::tempdir().expect("home");
    let unreachable = fixture::dead();
    let offline = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args([
            "configuration",
            "install",
            repo.path().to_str().expect("repo"),
        ])
        .env("PLUMB_HOME", home.path())
        .env("PLUMB_RULES_SOURCE", &unreachable)
        .env_remove("PLUMB_CONFIGURATION")
        .output()
        .expect("install");
    assert!(
        offline.status.success(),
        "{}",
        String::from_utf8_lossy(&offline.stderr)
    );
    assert!(!home.path().join("depot").exists());

    let (listener, source) = fixture::Mock::open();
    let held = fixture::bundle("v0.163.0", Some(b"precache\n"));
    let mut routes = std::collections::BTreeMap::new();
    let pointer = fixture::publish(&mut routes, &source, &held);
    let _server = fixture::Mock::start(listener, routes);
    let pin = plumb::depot::v3::Pinned::pointer(&pointer)
        .expect("pin")
        .encode()
        .expect("encode");

    let warm = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args([
            "configuration",
            "install",
            repo.path().to_str().expect("repo"),
        ])
        .env("PLUMB_HOME", home.path())
        .env("PLUMB_RULES_SOURCE", &source)
        .env("PLUMB_CONFIGURATION", &pin)
        .output()
        .expect("warm install");
    assert!(
        warm.status.success(),
        "{}",
        String::from_utf8_lossy(&warm.stderr)
    );
    assert!(
        home.path()
            .join("depot/generations")
            .join(&pointer.generation)
            .join(plumb::depot::v3::LEAF)
            .is_file()
    );

    let bad = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args([
            "configuration",
            "install",
            repo.path().to_str().expect("repo"),
        ])
        .env("PLUMB_HOME", home.path())
        .env("PLUMB_RULES_SOURCE", &source)
        .env("PLUMB_CONFIGURATION", "not-json")
        .output()
        .expect("bad install");
    assert!(!bad.status.success());
}
