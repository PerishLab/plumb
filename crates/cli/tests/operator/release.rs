use super::fixture::{AWS, CURL, SPEC};
use std::path::Path;
use std::process::{Command, Output};

struct Fixture<'a> {
    root: &'a Path,
    tools: &'a Path,
}

fn run(command: &mut Command) -> Output {
    let output = command.output().expect("plumb should run");
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

struct Compile<'a> {
    fixture: &'a Fixture<'a>,
    artifacts: &'a Path,
    channel: &'a str,
    version: &'a str,
    out: &'a Path,
    promotion: Option<&'a Path>,
    commit: &'a str,
}

fn compile(input: Compile<'_>) {
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

impl Fixture<'_> {
    fn command(&self) -> Command {
        let mut held = Command::new(env!("CARGO_BIN_EXE_plumb"));
        let path = format!(
            "{}:{}",
            self.tools.display(),
            std::env::var("PATH").unwrap_or_default()
        );
        held.env("PATH", path)
            .env("FAKE_S3_ROOT", self.root)
            .env("PLUMB_RELEASE_ROOT", self.root);
        held
    }

    fn archive(&self, artifacts: &Path, version: &str) {
        use std::os::unix::fs::PermissionsExt;

        let seat = self.root.join("binary");
        std::fs::create_dir_all(&seat).expect("binary root");
        let binary = seat.join("probe");
        std::fs::write(&binary, format!("#!/bin/sh\nprintf 'probe {version}\\n'\n"))
            .expect("probe binary");
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755))
            .expect("binary mode");
        run(Command::new("tar").args([
            "-C",
            seat.to_str().expect("binary root path"),
            "-czf",
            artifacts
                .join("probe-x86_64-unknown-linux-gnu.tar.gz")
                .to_str()
                .expect("artifact path"),
            "probe",
        ]));
    }

    fn seed(&self) {
        use std::os::unix::fs::PermissionsExt;

        std::fs::write(self.root.join("plumb.toml"), SPEC).expect("release manifest");
        run(Command::new("git")
            .arg("-C")
            .arg(self.root)
            .args(["init", "-q"]));
        run(Command::new("git")
            .arg("-C")
            .arg(self.root)
            .args(["config", "user.name", "Fixture"]));
        run(Command::new("git").arg("-C").arg(self.root).args([
            "config",
            "user.email",
            "fixture@example.test",
        ]));
        for (name, text) in [("aws", AWS), ("curl", CURL)] {
            let path = self.tools.join(name);
            std::fs::write(&path, text).expect("fake tool");
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
                .expect("tool mode");
        }
    }

    fn changelog(&self, version: &str) {
        let seat = self.root.join("docs/CHANGELOG").join(version);
        for language in ["en", "zh"] {
            std::fs::create_dir_all(seat.join(language)).expect("changelog language root");
            for leaf in ["INDEX.md", "MIGRATION.md"] {
                std::fs::write(seat.join(language).join(leaf), "complete\n")
                    .expect("changelog leaf");
            }
        }
    }

    fn candidate(&self) -> String {
        run(Command::new("git")
            .arg("-C")
            .arg(self.root)
            .args(["add", "plumb.toml", "docs"]));
        run(Command::new("git")
            .arg("-C")
            .arg(self.root)
            .args(["commit", "-qm", "candidate"]));
        String::from_utf8(
            run(Command::new("git")
                .arg("-C")
                .arg(self.root)
                .args(["rev-parse", "HEAD"]))
            .stdout,
        )
        .expect("candidate utf8")
        .trim()
        .to_string()
    }
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
    fixture.changelog("v1.2.0");
    let candidate = fixture.candidate();
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
        .env("PLUMB_RELEASE_PROMOTION_CHANNEL", "beta")
        .env("PLUMB_RELEASE_PROMOTION_VERSION", "v1.2.0-beta.7")
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
