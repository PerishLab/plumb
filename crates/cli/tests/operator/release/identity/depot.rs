use super::super::world::{Fixture, run};
use super::marker::{seal, seeded, stamp};
use sha2::{Digest, Sha256};
use std::os::unix::fs::PermissionsExt as _;
use std::process::Command;

#[test]
fn generation() {
    let temp = tempfile::tempdir().expect("temp root");
    let bare = tempfile::tempdir().expect("bare root");
    let tools = temp.path().join("tools");
    std::fs::create_dir(&tools).expect("tool root");
    let fixture = Fixture {
        root: temp.path(),
        tools: &tools,
    };
    seeded(&fixture, bare.path());
    let manifest = std::fs::read_to_string(fixture.root.join("plumb.toml")).expect("manifest");
    std::fs::write(
        fixture.root.join("plumb.toml"),
        format!(
            "{manifest}\n[release.depot]\nsource = \"https://depot.test\"\nderivatives = [\"skill\"]\n"
        ),
    )
    .expect("depot declaration");
    run(Command::new("git")
        .arg("-C")
        .arg(fixture.root)
        .args(["add", "plumb.toml"]));
    run(Command::new("git")
        .arg("-C")
        .arg(fixture.root)
        .args(["commit", "-qm", "declare depot"]));
    for reference in ["HEAD:refs/heads/main", "HEAD:refs/heads/release/v1.2.0"] {
        run(Command::new("git")
            .arg("-C")
            .arg(fixture.root)
            .args(["push", "-q", "origin", reference]));
    }
    let commit = String::from_utf8(
        run(Command::new("git")
            .arg("-C")
            .arg(fixture.root)
            .args(["rev-parse", "HEAD"]))
        .stdout,
    )
    .expect("commit")
    .trim()
    .to_string();
    stamp(fixture.root, "v1.2.0-beta.1", &commit, true);
    seal(fixture.root, &commit);
    let source = fixture.root.join("media/skill");
    std::fs::create_dir_all(&source).expect("skill source");
    std::fs::write(source.join("SKILL.md"), "# Probe\n").expect("skill");

    for _ in 0..2 {
        let output = fixture
            .command()
            .current_dir(fixture.root)
            .env("PLUMB_DEPOT_AUTHORITY_ACCESS", "access")
            .env("PLUMB_DEPOT_AUTHORITY_SECRET", "secret")
            .env("PLUMB_DEPOT_AUTHORITY_BUCKET", "depot")
            .env("PLUMB_DEPOT_AUTHORITY_ENDPOINT", "https://s3.test")
            .args([
                "depot",
                "skill",
                ".",
                "--marker",
                "v1.2.0-beta.1",
                "--from",
                source.to_str().expect("source"),
            ])
            .output()
            .expect("depot publish");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("generation"));
    }
    let base = fixture
        .root
        .join("depot/channels/beta/skills/versions/v1.2.0-beta.1");
    let pointer: serde_json::Value =
        serde_json::from_slice(&std::fs::read(base.join("latest.json")).expect("latest pointer"))
            .expect("pointer");
    assert_eq!(pointer["marker"]["name"], "v1.2.0-beta.1");
    assert_eq!(pointer["version"], "v1.2.0-beta.1");
    assert!(
        base.join("generations")
            .join(pointer["generation"].as_str().expect("generation"))
            .is_dir()
    );
}

#[test]
fn configuration() {
    let temp = tempfile::tempdir().expect("temp root");
    let bare = tempfile::tempdir().expect("bare root");
    let tools = temp.path().join("tools");
    std::fs::create_dir(&tools).expect("tool root");
    let fixture = Fixture {
        root: temp.path(),
        tools: &tools,
    };
    seeded(&fixture, bare.path());
    let manifest = std::fs::read_to_string(fixture.root.join("plumb.toml")).expect("manifest");
    std::fs::write(
        fixture.root.join("plumb.toml"),
        format!(
            "{manifest}\n[release.depot]\nsource = \"https://depot.test\"\nderivatives = [\"configuration\"]\nvalidator = [\"probe\", \"doctor\"]\n"
        ),
    )
    .expect("depot declaration");
    run(Command::new("git")
        .arg("-C")
        .arg(fixture.root)
        .args(["add", "plumb.toml"]));
    run(Command::new("git").arg("-C").arg(fixture.root).args([
        "commit",
        "-qm",
        "declare configuration",
    ]));
    for reference in ["HEAD:refs/heads/main", "HEAD:refs/heads/release/v1.2.0"] {
        run(Command::new("git")
            .arg("-C")
            .arg(fixture.root)
            .args(["push", "-q", "origin", reference]));
    }
    let commit = String::from_utf8(
        run(Command::new("git")
            .arg("-C")
            .arg(fixture.root)
            .args(["rev-parse", "HEAD"]))
        .stdout,
    )
    .expect("commit")
    .trim()
    .to_string();
    stamp(fixture.root, "v1.2.0-beta.1", &commit, true);
    seal(fixture.root, &commit);
    validator(fixture.root);
    let pointer = fixture.root.join("releases/v1/channels/beta.json");
    std::fs::create_dir_all(pointer.parent().expect("channel parent")).expect("channel root");
    std::fs::write(
        pointer,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema": 1,
            "product": "probe",
            "channel": "beta",
            "releaseVersion": "v1.2.0-beta.1",
            "commit": commit,
            "seal": {
                "name": "seal.json",
                "mime": "application/json; charset=utf-8",
                "sha256": "0".repeat(64),
                "size": 0,
                "url": "https://releases.test/v1/releases/beta/v1.2.0-beta.1/seal.json"
            },
            "managers": {}
        }))
        .expect("channel pointer"),
    )
    .expect("channel pointer write");
    stamp(fixture.root, "v1.2.0-beta.2", &commit, true);
    let source = fixture.root.join("media/configuration");
    std::fs::create_dir_all(source.join("rules")).expect("configuration source");
    std::fs::write(source.join("rules/probe.toml"), "answer = 42\n").expect("configuration");

    let output = fixture
        .command()
        .current_dir(fixture.root)
        .env("PLUMB_DEPOT_AUTHORITY_ACCESS", "access")
        .env("PLUMB_DEPOT_AUTHORITY_SECRET", "secret")
        .env("PLUMB_DEPOT_AUTHORITY_BUCKET", "depot")
        .env("PLUMB_DEPOT_AUTHORITY_ENDPOINT", "https://s3.test")
        .args([
            "depot",
            "configuration",
            ".",
            "--marker",
            "v1.2.0-beta.2",
            "--from",
            source.to_str().expect("source"),
        ])
        .output()
        .expect("configuration publish");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let base = fixture
        .root
        .join("depot/channels/beta/configurations/versions/v1.2.0-beta.2");
    assert!(base.join("latest.json").is_file());
    assert!(!fixture.root.join("depot/v2").exists());

    stamp(fixture.root, "v1.2.0-beta.3", &commit, true);
    let recovery = fixture.root.join("validator/probe");
    let output = fixture
        .command()
        .current_dir(fixture.root)
        .env("PLUMB_DEPOT_AUTHORITY_ACCESS", "access")
        .env("PLUMB_DEPOT_AUTHORITY_SECRET", "secret")
        .env("PLUMB_DEPOT_AUTHORITY_BUCKET", "depot")
        .env("PLUMB_DEPOT_AUTHORITY_ENDPOINT", "https://s3.test")
        .args([
            "depot",
            "configuration",
            ".",
            "--marker",
            "v1.2.0-beta.3",
            "--from",
            source.to_str().expect("source"),
            "--recovery-validator",
            recovery.to_str().expect("recovery validator"),
        ])
        .output()
        .expect("configuration recovery publish");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        fixture
            .root
            .join("depot/channels/beta/configurations/versions/v1.2.0-beta.3/latest.json")
            .is_file()
    );
}

fn validator(root: &std::path::Path) {
    let stage = root.join("validator");
    std::fs::create_dir(&stage).expect("validator stage");
    let binary = stage.join("probe");
    std::fs::write(
        &binary,
        "#!/bin/sh\n[ \"$1\" = --version ] && { echo 'probe v1.2.0-beta.1'; exit; }\n[ \"$1\" = doctor ] && [ -f \"$PROBE_DEPOT_SNAPSHOT/manifest.toml\" ] && [ -f \"$PROBE_DEPOT_SNAPSHOT/rules/probe.toml\" ]\n",
    )
    .expect("validator");
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).expect("mode");
    let archive = root.join("probe-x86_64-unknown-linux-gnu.tar.gz");
    run(Command::new("tar").args([
        "-C",
        stage.to_str().expect("stage"),
        "-czf",
        archive.to_str().expect("archive"),
        "probe",
    ]));
    let bytes = std::fs::read(&archive).expect("archive bytes");
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let name = archive
        .file_name()
        .and_then(|held| held.to_str())
        .expect("name");
    let object = root
        .join("releases/v1/objects/sha256")
        .join(&digest)
        .join(name);
    std::fs::create_dir_all(object.parent().expect("object parent")).expect("object root");
    std::fs::write(&object, &bytes).expect("published validator");
    let seal = root.join("releases/v1/releases/beta/v1.2.0-beta.1/seal.json");
    let mut held: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&seal).expect("seal")).expect("seal json");
    held["artifacts"]["linux-x64"] = serde_json::json!({
        "name": name,
        "mime": "application/gzip",
        "sha256": digest,
        "size": bytes.len(),
        "url": format!("https://releases.test/v1/objects/sha256/{digest}/{name}"),
    });
    std::fs::write(seal, serde_json::to_vec_pretty(&held).expect("seal body")).expect("seal write");
}
