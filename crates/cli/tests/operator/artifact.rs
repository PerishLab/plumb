use super::fixture::SPEC;
use std::path::{Path, PathBuf};
use std::process::Command;

const COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

struct Fixture {
    root: tempfile::TempDir,
}

struct Cut<'a> {
    channel: &'a str,
    version: &'a str,
    label: &'a str,
    proof: Option<&'a Path>,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().expect("fixture root");
        std::fs::write(root.path().join("plumb.toml"), SPEC).expect("release manifest");
        Self { root }
    }

    fn command(&self) -> Command {
        let mut held = Command::new(env!("CARGO_BIN_EXE_plumb"));
        held.env("PLUMB_RELEASE_ROOT", self.root.path());
        held
    }

    fn docs(&self) -> PathBuf {
        let seat = self.root.path().join("docs/CHANGELOG/v1.2.0");
        for tongue in ["en", "zh"] {
            std::fs::create_dir_all(seat.join(tongue)).expect("changelog root");
            for leaf in ["INDEX.md", "MIGRATION.md"] {
                std::fs::write(seat.join(tongue).join(leaf), "complete\n").expect("changelog");
            }
        }
        let artifacts = seat.join("artifacts");
        std::fs::create_dir(&artifacts).expect("version artifact root");
        std::fs::write(artifacts.join("migration.sh"), "#!/bin/sh\n").expect("Unix artifact");
        std::fs::write(artifacts.join("migration.ps1"), "Write-Output ready\n")
            .expect("Windows artifact");
        std::fs::write(artifacts.join("opaque.bin"), [0, 1, 2, 3]).expect("opaque artifact");
        artifacts
    }

    fn cut(&self, input: Cut<'_>) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;

        let artifacts = self.root.path().join(format!("{}-artifacts", input.label));
        std::fs::create_dir(&artifacts).expect("artifact root");
        let binary = self.root.path().join("probe");
        std::fs::write(&binary, "#!/bin/sh\n").expect("probe binary");
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755))
            .expect("binary mode");
        pass(
            Command::new("tar").args([
                "-czf",
                artifacts
                    .join("probe-x86_64-unknown-linux-gnu.tar.gz")
                    .to_str()
                    .expect("archive path"),
                "-C",
                self.root.path().to_str().expect("fixture path"),
                "probe",
            ]),
        );
        pass(
            self.command()
                .args(["release", "assemble"])
                .env("PLUMB_RELEASE_VERSION", input.version)
                .env("PLUMB_RELEASE_ARTIFACTS", &artifacts),
        );
        let out = self.root.path().join(input.label);
        let mut command = self.command();
        command
            .args(["release", "compile"])
            .env("PLUMB_RELEASE_CHANNEL", input.channel)
            .env("PLUMB_RELEASE_VERSION", input.version)
            .env("PLUMB_RELEASE_COMMIT", COMMIT)
            .env("PLUMB_RELEASE_ARTIFACTS", &artifacts)
            .env("PLUMB_RELEASE_OUTPUT", &out);
        if let Some(path) = input.proof {
            command.env("PLUMB_RELEASE_PROMOTION", path);
        }
        pass(&mut command);
        out
    }
}

fn pass(command: &mut Command) {
    let output = command.output().expect("command should run");
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn seal(path: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(path).expect("seal")).expect("seal json")
}

#[test]
fn sealed() {
    let fixture = Fixture::new();
    let source = fixture.docs();
    let beta = fixture.cut(Cut {
        channel: "beta",
        version: "v1.2.0-beta.7",
        label: "beta",
        proof: None,
    });
    let stable = fixture.cut(Cut {
        channel: "stable",
        version: "v1.2.0",
        label: "stable",
        proof: Some(&beta.join("seal.json")),
    });
    let candidate = seal(&beta.join("seal.json"));
    let permanent = seal(&stable.join("seal.json"));
    for key in ["migration.sh", "migration.ps1", "opaque.bin"] {
        assert_eq!(
            candidate["artifacts"][key]["sha256"],
            permanent["artifacts"][key]["sha256"]
        );
        assert_eq!(
            candidate["artifacts"][key]["mime"],
            "application/octet-stream"
        );
        assert!(source.join(key).is_file());
    }
}

#[test]
fn refuses() {
    let fixture = Fixture::new();
    let seat = fixture.root.path().join("docs/CHANGELOG/v1.2.0/artifacts");
    std::fs::create_dir_all(seat.join("nested")).expect("nested artifact");
    let artifacts = fixture.root.path().join("artifacts");
    std::fs::create_dir(&artifacts).expect("artifact root");
    let output = fixture
        .command()
        .args(["release", "assemble"])
        .env("PLUMB_RELEASE_VERSION", "v1.2.0-beta.1")
        .env("PLUMB_RELEASE_ARTIFACTS", &artifacts)
        .output()
        .expect("plumb should run");
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success() && error.contains("not a regular file"),
        "{error}"
    );
}

#[test]
fn collides() {
    let fixture = Fixture::new();
    let seat = fixture.root.path().join("docs/CHANGELOG/v1.2.0/artifacts");
    std::fs::create_dir_all(&seat).expect("version artifact root");
    std::fs::write(seat.join("linux-x64"), "collision\n").expect("version artifact");
    let artifacts = fixture.root.path().join("artifacts");
    std::fs::create_dir(&artifacts).expect("artifact root");
    let output = fixture
        .command()
        .args(["release", "assemble"])
        .env("PLUMB_RELEASE_VERSION", "v1.2.0-beta.1")
        .env("PLUMB_RELEASE_ARTIFACTS", &artifacts)
        .output()
        .expect("plumb should run");
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success() && error.contains("collides"),
        "{error}"
    );
}
