use super::world::{Fixture, SPEC, run};
use std::path::Path;
use std::process::Command;

#[path = "image/binary.rs"]
mod binary;
#[path = "../inputs.rs"]
mod inputs;

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
    held.args(["ship", "compile"])
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
    let bucket = "perish-probe-releases";
    command
        .env(format!("PLUMB_{operation}_ACCESS"), "access")
        .env(format!("PLUMB_{operation}_SECRET"), "secret")
        .env(format!("PLUMB_{operation}_BUCKET"), bucket)
        .env(format!("PLUMB_{operation}_ENDPOINT"), "https://s3.test");
    if operation == "PUBLISH" {
        command.env("PLUMB_RELEASE_CAPSULE", capsule).env(
            "PLUMB_PUBLISH_FINGERPRINT",
            plumb::depot::sha(b"https://s3.test"),
        );
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
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\n[workspace.package]\nversion='1.2.0'\n",
    )
    .unwrap();
    let home = tempfile::tempdir().unwrap();
    let manifest =
        format!("[[layout.file]]\nname=['Cargo.toml']\nrule=['rule://seat/compiler']\n{SPEC}");
    let binding = crate::marker::prepare(root, home.path(), &manifest, "v1.2.0-beta.7");
    let candidate = run(Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "HEAD"]));
    let candidate = String::from_utf8(candidate.stdout)
        .unwrap()
        .trim()
        .to_string();
    super::image::executable(
        &tools.join("git"),
        "#!/bin/sh\nfor arg in \"$@\"; do [ \"$arg\" = fetch ] && exit 0; done\nexec /usr/bin/git \"$@\"\n",
    );
    let command = || {
        let mut held = fixture.command();
        held.current_dir(root)
            .env("PLUMB_HOME", home.path())
            .env("PLUMB_RULES_SOURCE", "https://depot.test");
        held
    };
    let inventory = crate::support::Bucket::open(18);
    let source = root.join("binary");
    std::fs::create_dir_all(&source).unwrap();
    binary::archive(
        &command,
        &source.join("probe-x86_64-unknown-linux-gnu.tar.gz"),
        "v1.2.0-beta.7",
    );
    let workload = inputs::workload(
        &mut command(),
        &source.join("probe-x86_64-unknown-linux-gnu.tar.gz"),
        1,
    );
    let original = std::fs::read_to_string(tools.join("curl")).unwrap();
    super::image::executable(&tools.join("curl"), &original.replace(
        "case \"$url\" in", "case \"$url\" in\n  https://inventory.invalid/binary.tar.gz) touch \"$FAKE_S3_ROOT/fetched\"; path=\"$FAKE_S3_ROOT/binary/probe-x86_64-unknown-linux-gnu.tar.gz\" ;;"));
    let request = |version: &str| {
        let mut workload = workload.clone();
        workload["receipt"]["artifact"] = serde_json::json!(plumb::depot::sha(
            &std::fs::read(source.join("probe-x86_64-unknown-linux-gnu.tar.gz")).unwrap()
        ));
        let request = serde_json::json!({
            "schema":"plumb.ship-request/v2", "configuration":binding["configuration"], "profile":binding["profile"],
            "action":"ship/binary", "projections":["Cargo.toml#/workspace/package/version"], "roots":["Cargo.toml","plumb.toml"],
        "operation":{"type":"publication","workloads":[workload]},
            "keys":{"workload":plumb::depot::sha(version.as_bytes()),"proof":"2".repeat(64),"publication":plumb::depot::sha(version.as_bytes())},
        });
        crate::marker::planned(root, home.path(), request, version)
    };
    let publish = |version: &str, out: &Path, request: &serde_json::Value| {
        let mut held = command();
        held.args(["ship", "execute", "--request", &request.to_string()])
            .env("PLUMB_RELEASE_VERSION", version)
            .env("PLUMB_RELEASE_OUTPUT", out)
            .env("PLUMB_RELEASE_ARTIFACTS", &artifacts)
            .env("PLUMB_WORKFLOW_INVENTORY_ACCESS", "access")
            .env("PLUMB_WORKFLOW_INVENTORY_SECRET", "secret")
            .env("PLUMB_WORKFLOW_INVENTORY_BUCKET", "workflow")
            .env("PLUMB_WORKFLOW_INVENTORY_ENDPOINT", inventory.endpoint())
            .env(
                "PLUMB_WORKFLOW_INVENTORY_URL",
                "https://inventory.invalid/inventory.json",
            );
        authority(&mut held, &out.join("capsule.json"), "PUBLISH");
        held
    };
    fixture.archive(&artifacts, "v1.2.0-beta.7");
    inputs::refuses(
        &|request| publish("v1.2.0-beta.7", &root.join("refused"), request),
        &request("v1.2.0-beta.7"),
        root,
    );

    let beta = root.join("beta");
    let stray = root.join("nonstable-must-not-consume-promotion.json");
    for attempt in 0..2 {
        let output = if attempt == 0 {
            beta.clone()
        } else {
            root.join("beta-retry")
        };
        let mut held = publish("v1.2.0-beta.7", &output, &request("v1.2.0-beta.7"));
        held.env("PLUMB_RELEASE_PROMOTION", &stray);
        if attempt == 1 {
            held.env("FAKE_S3_GET_FAILURE", "true");
        }
        run(&mut held);
    }
    let proof = root.join("promotion/nested/seal.json");
    run(command()
        .args(["ship", "promote"])
        .env("PLUMB_RELEASE_CHANNEL", "stable")
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
    run(command()
        .args(["ship", "binary", "smoke"])
        .env("PLUMB_RELEASE_URL", manager)
        .env("PLUMB_RELEASE_VERSION", "v1.2.0-beta.7"));

    let annotation = serde_json::json!({
        "schema":"plumb.release-marker/v2","product":"probe","marker":"v1.2.0",
        "configuration":{"channel":"stable","version":plumb::version!("PLUMB").to_string(),"generation":binding["configuration"]},
        "profile":binding["profile"],
    });
    run(Command::new("git").arg("-C").arg(root).args([
        "tag",
        "-a",
        "v1.2.0",
        "-m",
        &annotation.to_string(),
    ]));
    let stable = root.join("stable");
    binary::archive(
        &command,
        &source.join("probe-x86_64-unknown-linux-gnu.tar.gz"),
        "v1.2.0",
    );
    let record = stable.join("capsule.json");
    run(publish("v1.2.0", &stable, &request("v1.2.0")).env(
        "PLUMB_RELEASE_PROMOTION",
        root.join("stable-promotion.json"),
    ));
    let mut wrong = command();
    wrong.args(["depot", "managers", "--marker", "v1.2.0"]);
    authority(&mut wrong, &record, "ACTIVATE");
    let wrong = wrong
        .env("PLUMB_ACTIVATE_BUCKET", "perish-another-releases")
        .output()
        .expect("wrong activation target should be inspected");
    assert!(!wrong.status.success());
    assert!(String::from_utf8_lossy(&wrong.stderr).contains(
        "activation authority targets perish-another-releases, not perish-probe-releases"
    ));
    for _ in 0..2 {
        let mut activate = command();
        activate.args(["depot", "channel", "--marker", "v1.2.0"]);
        authority(&mut activate, &record, "ACTIVATE");
        run(&mut activate);
        let mut project = command();
        project.args(["depot", "managers", "--marker", "v1.2.0"]);
        authority(&mut project, &record, "ACTIVATE");
        run(&mut project);
    }

    let mut verify = command();
    verify
        .args(["ship", "binary", "verify"])
        .env("PLUMB_RELEASE_CAPSULE", &record)
        .env("PLUMB_RELEASE_ACTIVATED", "true");
    run(&mut verify);
    run(command().args(["ship", "inspect"]).env(
        "PLUMB_RELEASE_URL",
        "https://releases.test/v1/releases/beta/v1.2.0-beta.7/seal.json",
    ));
    run(command().args(["ship", "binary", "inspect"]).env(
        "PLUMB_RELEASE_URL",
        "https://releases.test/v1/releases/beta/v1.2.0-beta.7/seal.json",
    ));
    run(command()
        .args(["ship", "inspect"])
        .env(
            "PLUMB_RELEASE_URL",
            "https://releases.test/v1/channels/stable.json",
        )
        .env("PLUMB_RELEASE_ACTIVATED", "true"));
    run(command()
        .args(["ship", "binary", "inspect"])
        .env(
            "PLUMB_RELEASE_URL",
            "https://releases.test/v1/channels/stable.json",
        )
        .env("PLUMB_RELEASE_ACTIVATED", "true"));
    assert!(
        root.join("perish-probe-releases/v1/channels/stable.json")
            .is_file()
    );
    assert!(root.join("perish-probe-releases/manage.sh").is_file());
    inventory.finish();
}

#[test]
fn intent() {
    let temp = tempfile::tempdir().expect("temp root");
    let root = temp.path();
    let out = root.join("managers");
    super::support::stock(&root.join("configurations"), &[]);
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
    assert!(manager.contains("if [ \"probe\" = plumb ]; then"));
    assert!(!out.join("canonical").exists());
}

#[test]
fn configures() {
    let temp = tempfile::tempdir().expect("temp root");
    let root = temp.path();
    let out = root.join("managers");
    super::support::stock(&root.join("configurations"), &[]);
    std::fs::write(
        root.join("plumb.toml"),
        SPEC.replace("product = \"probe\"", "product = \"plumb\""),
    )
    .expect("release manifest");
    let fixture = Fixture { root, tools: root };
    run(fixture
        .command()
        .args(["ship", "binary", "managers"])
        .env("PLUMB_RELEASE_CHANNEL", "stable")
        .env("PLUMB_RELEASE_VERSION", "v1.2.0")
        .env("PLUMB_RELEASE_OUTPUT", &out));
    let manager = std::fs::read_to_string(out.join("manage.sh")).expect("manager");
    assert!(manager.contains("if [ \"plumb\" = plumb ]; then"));
    assert!(manager.contains("\"$LOCAL_BIN_DIR/plumb\" configuration install"));
}
