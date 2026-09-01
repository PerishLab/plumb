use super::super::world::{Fixture, run};
use super::marker::{seal, seeded, stamp};
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
