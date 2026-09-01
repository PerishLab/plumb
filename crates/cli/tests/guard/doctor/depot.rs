use std::path::Path;
use std::process::Command;

fn run(root: &Path, seat: &Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["doctor", root.to_str().expect("path should be utf8")])
        .env_remove("PLUMB_RELEASE_VERSION")
        .env("PLUMB_HOME", seat)
        .output()
        .expect("plumb should run");
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn stock(seat: &Path, mark: &str, manifest: &str) {
    let seat = seat.join("configurations");
    std::fs::create_dir_all(seat.join(mark)).expect("seat should be made");
    std::fs::write(seat.join(mark).join("plumb.toml"), manifest).expect("manifest");
    std::fs::write(
        seat.join("metadata.json"),
        format!(
            "{{\"format\":1,\"product\":\"plumb\",\"channel\":\"stable\",\"version\":\"{mark}\",\"source\":\"https://depot.plumb.perish.uk\",\"commit\":\"\"}}"
        ),
    )
    .expect("pointer");
}

#[test]
fn absent() {
    let fixture = super::fixture();
    let empty = tempfile::tempdir().expect("seat");
    let out = run(fixture.path(), empty.path());
    assert!(out.contains("cannot read installed rules"), "{out}");
    assert!(out.contains("run plumb configuration install"), "{out}");
}

#[test]
fn floor() {
    let fixture = super::fixture();
    let seat = tempfile::tempdir().expect("seat");
    stock(
        seat.path(),
        "29990101T000000Z",
        "[schema]\nformat = 1\nversion = \"v99.0.0\"\n\n[metadata]\nversion = \"29990101T000000Z\"\nsource = \"https://depot.plumb.perish.uk\"\nchannel = \"stable\"\ncommit = \"\"\n",
    );
    let out = run(fixture.path(), seat.path());
    assert!(out.contains("declares a floor of v99.0.0"), "{out}");
}

#[test]
fn drift() {
    let fixture = super::fixture();
    let seat = super::super::support::depot(&[]);
    std::fs::write(
        seat.path()
            .join("configurations/29990101T000000Z/rules/structure.toml"),
        "drift",
    )
    .expect("drift rule");
    let out = run(fixture.path(), seat.path());
    assert!(
        out.contains("depot object drift: rules/structure.toml"),
        "{out}"
    );
}

fn stage(home: &tempfile::TempDir, version: &str) -> std::path::PathBuf {
    let root = home.path().join("configurations");
    let legacy = plumb::depot::Seat::at(&root).expect("legacy fixture");
    let release = plumb::depot::v2::Release {
        product: "plumb".into(),
        channel: "stable".into(),
        version: version.into(),
        commit: "1".repeat(40),
        seal: plumb::depot::v2::Seal {
            url: "https://releases.plumb.perish.uk/v1/releases/stable/v0.0.1/seal.json".into(),
            sha256: "2".repeat(64),
        },
    };
    let manifest = plumb::depot::v2::Manifest {
        format: plumb::depot::v2::FORMAT,
        source: "https://depot.plumb.perish.uk".into(),
        derivative: plumb::depot::v2::Kind::Configuration,
        release: release.clone(),
        snapshot: plumb::depot::v2::Snapshot {
            timestamp: "20260831T000000Z".into(),
            commit: "3".repeat(40),
        },
        objects: legacy.manifest().objects.clone(),
    };
    let body = manifest.encode().expect("manifest");
    let pointer = plumb::depot::v2::Pointer::new(&manifest, body.as_bytes()).expect("pointer");
    let seat = plumb::depot::v2::local(&root, &release, "20260831T000000Z").expect("seat");
    std::fs::create_dir_all(&seat).expect("seat root");
    for object in &manifest.objects {
        let target = seat.join(&object.path);
        std::fs::create_dir_all(target.parent().expect("object parent"))
            .expect("object parent seat");
        std::fs::write(
            target,
            legacy.read(&object.path).expect("legacy object").as_bytes(),
        )
        .expect("v2 object");
    }
    std::fs::write(seat.join(plumb::depot::v2::LEAF), body).expect("manifest seat");
    std::fs::write(
        root.join(plumb::depot::v2::POINTER),
        pointer.encode().expect("pointer body"),
    )
    .expect("pointer seat");
    seat
}

#[test]
fn exact() {
    let fixture = super::fixture();
    let home = super::super::support::depot(&[]);
    stage(&home, "v0.0.1");
    let out = run(fixture.path(), home.path());
    assert!(
        out.contains(&format!(
            "belongs to Plumb v0.0.1, not the running v{}",
            env!("CARGO_PKG_VERSION")
        )),
        "{out}"
    );
}

#[test]
fn staged() {
    let fixture = super::fixture();
    let root = fixture.path();
    std::fs::write(root.join("plumb.toml"), "[layout]\n").expect("governance");
    for name in ["pre-commit", "commit-msg"] {
        std::fs::remove_file(root.join(".git/hooks").join(name)).expect("remove projected hook");
    }
    let source = super::super::support::depot(&[]);
    let snapshot = stage(&source, &format!("v{}", env!("CARGO_PKG_VERSION")));
    let home = tempfile::tempdir().expect("unrelated active depot home");
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["doctor", root.to_str().expect("fixture")])
        .env_remove("PLUMB_RELEASE_VERSION")
        .env("PLUMB_HOME", home.path())
        .env("PLUMB_DEPOT_SNAPSHOT", snapshot)
        .output()
        .expect("staged doctor");
    let out = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!out.contains("hook is absent"), "{out}");
    assert!(!out.contains("hooks/pre-commit is absent"), "{out}");
    assert!(!out.contains("active depot carries no"), "{out}");
}

#[test]
fn guarded() {
    let fixture = super::fixture();
    let root = fixture.path();
    std::fs::write(root.join("plumb.toml"), "[layout]\n").expect("governance");
    let seat = super::super::support::guard(&[], &format!("v{}", env!("CARGO_PKG_VERSION")));
    let home = tempfile::tempdir().expect("unrelated active depot home");
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["doctor", root.to_str().expect("fixture")])
        .env("PLUMB_HOME", home.path())
        .env("PLUMB_GUARD_CONFIGURATION", seat.path())
        .output()
        .expect("guarded doctor");
    let out = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!out.contains("cannot read installed rules"), "{out}");
    assert!(!out.contains("belongs to Plumb"), "{out}");
}

#[test]
fn legacy() {
    let fixture = super::fixture();
    let home = tempfile::tempdir().expect("empty home");
    let legacy = super::super::support::depot(&[]);
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["doctor", fixture.path().to_str().expect("fixture")])
        .env("PLUMB_HOME", home.path())
        .env("PLUMB_DEPOT_SEAT", legacy.path().join("configurations"))
        .output()
        .expect("plumb should run");
    let out = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(out.contains("cannot read installed rules"), "{out}");
}

#[test]
#[cfg(unix)]
fn managed() {
    use std::os::unix::fs::PermissionsExt as _;

    let fixture = super::fixture();
    let home = super::super::support::depot(&[]);
    let managed = tempfile::tempdir().expect("managed binary");
    let binary = managed.path().join("plumb");
    std::fs::copy(env!("CARGO_BIN_EXE_plumb"), &binary).expect("copy Plumb");
    std::fs::write(
        managed.path().join(".plumb-manager"),
        format!(
            "plumb-manager-v1\nauthority=https://releases.test\nchannel=stable\nversion=v{}\n",
            env!("CARGO_PKG_VERSION")
        ),
    )
    .expect("manager identity");
    let bin = managed.path().join("bin");
    std::fs::create_dir(&bin).expect("tool root");
    let curl = bin.join("curl");
    std::fs::write(
        &curl,
        "#!/bin/sh\nprintf '%s\\n' '{\"schema\":1,\"product\":\"plumb\",\"channel\":\"stable\",\"releaseVersion\":\"v99.0.0\"}'\n",
    )
    .expect("fake curl");
    std::fs::set_permissions(&curl, std::fs::Permissions::from_mode(0o755)).expect("curl mode");
    let output = Command::new(binary)
        .args(["doctor", fixture.path().to_str().expect("fixture")])
        .env("PLUMB_HOME", home.path())
        .env("PLUMB_RELEASES", "https://releases.test")
        .env(
            "PATH",
            format!(
                "{}:{}",
                bin.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .output()
        .expect("managed doctor");
    let out = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        out.contains(&format!(
            "the running Plumb is v{}, stable latest is v99.0.0",
            env!("CARGO_PKG_VERSION")
        )),
        "{out}"
    );
}
