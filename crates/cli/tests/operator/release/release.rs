use super::world::{Fixture, SPEC, run};
use std::path::Path;
use std::process::Command;

pub struct Compile<'a> {
    pub fixture: &'a Fixture<'a>,
    pub artifacts: &'a Path,
    pub channel: &'a str,
    pub version: &'a str,
    pub out: &'a Path,
    pub promotion: Option<&'a Path>,
    pub commit: &'a str,
}

pub fn compile(input: Compile<'_>) {
    let mut held = input.fixture.command();
    held.args(["release", "compile"])
        .env("PLUMB_RELEASE_CHANNEL", input.channel)
        .env("PLUMB_RELEASE_VERSION", input.version)
        .env("PLUMB_RELEASE_COMMIT", input.commit)
        .env("PLUMB_RELEASE_ARTIFACTS", input.artifacts)
        .env("PLUMB_RELEASE_OUTPUT", input.out);
    if let Some(path) = input.promotion {
        held.env("PLUMB_RELEASE_PROMOTION", path);
    }
    run(&mut held);
}

fn authority(command: &mut Command, capsule: &Path, operation: &str) {
    command
        .env(format!("PLUMB_{operation}_ACCESS"), "access")
        .env(format!("PLUMB_{operation}_SECRET"), "secret")
        .env(format!("PLUMB_{operation}_BUCKET"), "releases")
        .env(format!("PLUMB_{operation}_ENDPOINT"), "https://s3.test")
        .env("PLUMB_RELEASE_CAPSULE", capsule);
}

#[test]
fn cycle() {
    let temp = tempfile::tempdir().expect("temp root");
    let root = temp.path();
    let tools = root.join("tools");
    let artifacts = root.join("artifacts");
    std::fs::create_dir(&tools).expect("tool root");
    std::fs::create_dir(&artifacts).expect("artifact root");
    let fixture = Fixture {
        root,
        tools: &tools,
    };
    fixture.seed();
    let candidate = fixture.candidate();
    fixture.tag("v1.2.0-beta.7");
    fixture.archive(&artifacts, "v1.2.0-beta.7");

    let beta = root.join("beta");
    compile(Compile {
        fixture: &fixture,
        artifacts: &artifacts,
        channel: "beta",
        version: "v1.2.0-beta.7",
        out: &beta,
        promotion: None,
        commit: &candidate,
    });
    let manifest = beta.join("capsule.json");
    for _ in 0..2 {
        let mut held = fixture.command();
        held.args(["ship", "binary", "publish"]);
        authority(&mut held, &manifest, "PUBLISH");
        run(&mut held);
    }
    let proof = root.join("promotion/nested/seal.json");
    run(fixture
        .command()
        .args(["release", "promote"])
        .env("PLUMB_RELEASE_COMMIT", &candidate)
        .env("PLUMB_RELEASE_VERSION", "v1.2.0")
        .env("PLUMB_RELEASE_PROMOTION", &proof));
    assert!(proof.is_file());
    let seal: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(beta.join("seal.json")).expect("seal"))
            .expect("seal json");
    let manager = seal["managers"]["unix"]["url"]
        .as_str()
        .expect("manager url");
    run(fixture
        .command()
        .args(["ship", "binary", "smoke"])
        .env("PLUMB_RELEASE_URL", manager)
        .env("PLUMB_RELEASE_VERSION", "v1.2.0-beta.7"));

    fixture.archive(&artifacts, "v1.2.0");
    let stable = root.join("stable");
    compile(Compile {
        fixture: &fixture,
        artifacts: &artifacts,
        channel: "stable",
        version: "v1.2.0",
        out: &stable,
        promotion: Some(&proof),
        commit: &candidate,
    });
    let record = stable.join("capsule.json");
    let mut publish = fixture.command();
    publish.args(["ship", "binary", "publish"]);
    authority(&mut publish, &record, "PUBLISH");
    run(&mut publish);
    for _ in 0..2 {
        let mut project = fixture.command();
        project.args(["ship", "binary", "activate"]);
        authority(&mut project, &record, "ACTIVATE");
        run(&mut project);
        let mut activate = fixture.command();
        activate.args(["release", "activate"]);
        authority(&mut activate, &record, "ACTIVATE");
        run(&mut activate);
    }

    let mut verify = fixture.command();
    verify
        .args(["ship", "binary", "verify"])
        .env("PLUMB_RELEASE_CAPSULE", &record)
        .env("PLUMB_RELEASE_ACTIVATED", "true");
    run(&mut verify);
    run(fixture.command().args(["release", "inspect"]).env(
        "PLUMB_RELEASE_URL",
        "https://releases.test/v1/releases/beta/v1.2.0-beta.7/seal.json",
    ));
    run(fixture.command().args(["ship", "binary", "inspect"]).env(
        "PLUMB_RELEASE_URL",
        "https://releases.test/v1/releases/beta/v1.2.0-beta.7/seal.json",
    ));
    run(fixture
        .command()
        .args(["release", "inspect"])
        .env(
            "PLUMB_RELEASE_URL",
            "https://releases.test/v1/channels/stable.json",
        )
        .env("PLUMB_RELEASE_ACTIVATED", "true"));
    run(fixture
        .command()
        .args(["ship", "binary", "inspect"])
        .env(
            "PLUMB_RELEASE_URL",
            "https://releases.test/v1/channels/stable.json",
        )
        .env("PLUMB_RELEASE_ACTIVATED", "true"));
    assert!(root.join("releases/v1/channels/stable.json").is_file());
    assert!(root.join("releases/manage.sh").is_file());
}

#[test]
fn intent() {
    let temp = tempfile::tempdir().expect("temp root");
    let root = temp.path();
    let out = root.join("managers");
    std::fs::write(root.join("plumb.toml"), SPEC).expect("release manifest");
    let fixture = Fixture { root, tools: root };
    run(fixture
        .command()
        .args(["ship", "binary", "managers"])
        .env("PLUMB_RELEASE_CHANNEL", "canary")
        .env("PLUMB_RELEASE_VERSION", "v1.2.0-canary.9")
        .env("PLUMB_RELEASE_OUTPUT", &out));
    let manager = std::fs::read_to_string(out.join("manage.sh")).expect("manager");
    assert!(manager.contains("CHANNEL=${PROBE_CHANNEL:-canary}"));
    assert!(manager.contains("VERSION=${PROBE_VERSION:-v1.2.0-canary.9}"));
    assert!(!out.join("canonical").exists());
}
