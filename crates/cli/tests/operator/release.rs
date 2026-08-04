use super::fixture::{AWS, CURL, SPEC};
use std::path::Path;
use std::process::{Command, Output};

const COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const ATTACHMENT: &str =
    "[release.cargo]\nregistry = \"perish\"\npackages = [\"family-macro\", \"family-core\"]\n";
const WORKSPACE: &str = "[workspace]\nmembers = [\"crates/core\", \"crates/macro\", \"crates/helper\"]\nresolver = \"3\"\n\n[workspace.package]\nversion = \"0.10.2\"\nedition = \"2024\"\nlicense = \"MIT\"\nrepository = \"https://example.invalid/family\"\n\n[workspace.dependencies]\ncore-alias = { package = \"family-core\", path = \"crates/core\", version = \"=0.10.2\" }\nhelper = { path = \"crates/helper\", version = \"=9.9.9\" }\nregistry-core = { package = \"family-core\", version = \"=0.10.2\", registry = \"perish\" }\n\n[workspace.dependencies.family-macro]\npath = \"crates/macro\"\nversion = \"=0.10.2\"\n";
const CORE: &str = "[package]\nname = \"family-core\"\nversion.workspace = true\nedition.workspace = true\nlicense.workspace = true\nrepository.workspace = true\n\n[dependencies]\nmacro-alias = { package = \"family-macro\", path = \"../macro\", version = \"=0.10.2\" }\nhelper = { path = \"../helper\", version = \"=9.9.9\" }\n\n[build-dependencies.family-macro]\npath = \"../macro\"\nversion = \"=0.10.2\"\n\n[dev-dependencies]\ncore-alias = { package = \"family-core\", path = \".\", version = \"=0.10.2\" }\n";
const MACRO: &str = "[package]\nname = \"family-macro\"\nversion = \"0.10.2\"\nedition.workspace = true\nlicense.workspace = true\nrepository.workspace = true\n\n[dependencies.family-core]\npath = \"../core\"\nversion = \"=0.10.2\"\n\n[build-dependencies]\ncore-alias = { package = \"family-core\", path = \"../core\", version = \"=0.10.2\" }\n\n[dev-dependencies.family-core]\npath = \"../core\"\nversion = \"=0.10.2\"\n";
const HELPER: &str = "[package]\nname = \"helper\"\nversion = \"9.9.9\"\nedition.workspace = true\nlicense.workspace = true\nrepository.workspace = true\n";
const CARGO: &str = r#"#!/bin/sh
set -eu
if [ "$1" = metadata ]; then printf '%s\n' "{\"packages\":[{\"name\":\"family-core\",\"version\":\"0.10.2\",\"manifest_path\":\"$PWD/crates/core/Cargo.toml\",\"targets\":[]},{\"name\":\"family-macro\",\"version\":\"0.10.2\",\"manifest_path\":\"$PWD/crates/macro/Cargo.toml\",\"targets\":[]},{\"name\":\"helper\",\"version\":\"9.9.9\",\"manifest_path\":\"$PWD/crates/helper/Cargo.toml\",\"targets\":[]}],\"target_directory\":\"$PWD/target\"}"; exit 0; fi
[ "$(grep -c 'version = \"=0.10.2-beta.1\"' Cargo.toml)" -eq 2 ] && [ "$(grep -c 'version = \"=0.10.2-beta.1\"' crates/core/Cargo.toml)" -eq 3 ] && [ "$(grep -c 'version = \"=0.10.2-beta.1\"' crates/macro/Cargo.toml)" -eq 3 ] || { printf '%s\n' 'error: failed to select a version for requirement =0.10.2; candidate 0.10.2-beta.1 did not match' >&2; exit 101; }
grep -F 'version = "0.10.2-beta.1"' Cargo.toml >/dev/null && grep -F 'version = "0.10.2-beta.1"' crates/macro/Cargo.toml >/dev/null
grep -F 'helper = { path = "crates/helper", version = "=9.9.9" }' Cargo.toml >/dev/null && grep -F 'registry-core = { package = "family-core", version = "=0.10.2", registry = "perish" }' Cargo.toml >/dev/null && grep -F 'helper = { path = "../helper", version = "=9.9.9" }' crates/core/Cargo.toml >/dev/null && grep -F 'version = "9.9.9"' crates/helper/Cargo.toml >/dev/null
printf '0.10.2-beta.1\n' > cargo-observed
mkdir -p target/package/family-macro-0.10.2-beta.1
printf 'version = "0.10.2-beta.1"\n' > target/package/family-macro-0.10.2-beta.1/Cargo.toml
tar -czf target/package/family-macro-0.10.2-beta.1.crate -C target/package family-macro-0.10.2-beta.1
"#;

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
}

fn compile(input: Compile<'_>) {
    let mut held = input.fixture.command();
    held.args(["release", "compile"])
        .env("PLUMB_RELEASE_CHANNEL", input.channel)
        .env("PLUMB_RELEASE_VERSION", input.version)
        .env("PLUMB_RELEASE_COMMIT", COMMIT)
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
    fixture.archive(&artifacts, "v1.2.0-beta.7");

    let beta = root.join("beta");
    compile(Compile {
        fixture: &fixture,
        artifacts: &artifacts,
        channel: "beta",
        version: "v1.2.0-beta.7",
        out: &beta,
        promotion: None,
    });
    let manifest = beta.join("capsule.json");
    for _ in 0..2 {
        let mut held = fixture.command();
        held.args(["release", "publish"]);
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
        .args(["release", "smoke"])
        .env("PLUMB_RELEASE_URL", manager)
        .env("PLUMB_RELEASE_VERSION", "v1.2.0-beta.7"));

    fixture.archive(&artifacts, "v1.2.0");
    fixture.changelog("v1.2.0");
    let stable = root.join("stable");
    compile(Compile {
        fixture: &fixture,
        artifacts: &artifacts,
        channel: "stable",
        version: "v1.2.0",
        out: &stable,
        promotion: Some(&proof),
    });
    let record = stable.join("capsule.json");
    let mut publish = fixture.command();
    publish.args(["release", "publish"]);
    authority(&mut publish, &record, "PUBLISH");
    run(&mut publish);
    for _ in 0..2 {
        let mut activate = fixture.command();
        activate.args(["release", "activate"]);
        authority(&mut activate, &record, "ACTIVATE");
        run(&mut activate);
    }

    let mut verify = fixture.command();
    verify
        .args(["release", "verify"])
        .env("PLUMB_RELEASE_CAPSULE", &record)
        .env("PLUMB_RELEASE_ACTIVATED", "true");
    run(&mut verify);
    run(fixture.command().args(["release", "inspect"]).env(
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
        .args(["release", "managers"])
        .env("PLUMB_RELEASE_CHANNEL", "canary")
        .env("PLUMB_RELEASE_VERSION", "v1.2.0-canary.9")
        .env("PLUMB_RELEASE_OUTPUT", &out));
    let manager = std::fs::read_to_string(out.join("manage.sh")).expect("manager");
    assert!(manager.contains("CHANNEL=${PROBE_CHANNEL:-canary}"));
    assert!(manager.contains("VERSION=${PROBE_VERSION:-v1.2.0-canary.9}"));
    assert!(!out.join("canonical").exists());
}

#[test]
fn cargo() {
    use std::os::unix::fs::PermissionsExt;

    let root = tempfile::tempdir().expect("Cargo fixture");
    let path = root.path();
    let write = |path: &str, text: &str| {
        std::fs::write(root.path().join(path), text).expect("Cargo fixture file")
    };
    for package in ["core", "macro", "helper"] {
        std::fs::create_dir_all(path.join("crates").join(package)).expect("crate root");
    }
    write("plumb.toml", ATTACHMENT);
    write("Cargo.toml", WORKSPACE);
    write("crates/core/Cargo.toml", CORE);
    write("crates/macro/Cargo.toml", MACRO);
    write("crates/helper/Cargo.toml", HELPER);
    write("cargo", CARGO);
    let cargo = path.join("cargo");
    std::fs::set_permissions(&cargo, std::fs::Permissions::from_mode(0o755)).expect("Cargo mode");
    let fixture = Fixture {
        root: path,
        tools: path,
    };
    let refuse = || {
        fixture
            .command()
            .args(["release", "registry", "rehearse"])
            .env("PLUMB_RELEASE_VERSION", "v0.10.2-beta.1")
            .env_remove("PLUMB_RELEASE_REGISTRY_TOKEN")
            .output()
            .expect("plumb should run")
    };
    let output = refuse();
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success()
            && error.contains("PLUMB_RELEASE_REGISTRY_TOKEN is required")
            && !error.contains("missing field `product`"),
        "{error}"
    );
    let output = fixture
        .command()
        .args(["release", "registry", "rehearse"])
        .env("PLUMB_RELEASE_VERSION", "v0.10.2-beta.1")
        .env("PLUMB_RELEASE_REGISTRY_TOKEN", "secret")
        .output()
        .expect("plumb should run");
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{error}");
    let observed = std::fs::read_to_string(path.join("cargo-observed")).expect("observation");
    assert_eq!(observed, "0.10.2-beta.1\n");
}
