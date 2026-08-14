use super::fixture::AWS;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::process::{Command, Output};
const CURL: &str = r#"#!/bin/sh
set -eu
output=
url=
while [ $# -gt 0 ]; do
  case "$1" in
    --output|-o) output=$2; shift 2 ;;
    --retry|--retry-delay) shift 2 ;;
    --fail|--silent|--show-error|--location|--retry-all-errors) shift ;;
    *) url=$1; shift ;;
  esac
done
key=${url#https://releases.plumb.perish.uk/}
cp "$FAKE_S3_ROOT/releases/$key" "$output"
"#;
const COMMIT: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const SOURCE: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const AUTHORITY: &str = "https://releases.plumb.perish.uk";
const BETA: &str = "v0.20.0-beta.1";
const STABLE: &str = "v0.20.0";
struct Inspect<'a> {
    root: &'a Path,
    tools: &'a Path,
}
impl Inspect<'_> {
    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_plumb"));
        command
            .env("PATH", format!("{}:{}", self.tools.display(), env!("PATH")))
            .env("FAKE_S3_ROOT", self.root)
            .env("PLUMB_RELEASE_ROOT", self.root);
        command
    }
    fn seed(&self) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(
            self.root.join("plumb.toml"),
            format!(
                "[release]\nproduct = \"plumb\"\nauthority = \"{AUTHORITY}\"\nbinaries = [\"plumb\"]\ntargets = [\"x86_64-unknown-linux-gnu\"]\n"
            ),
        )
        .expect("manifest");
        for (name, text) in [("aws", AWS), ("curl", CURL)] {
            let path = self.tools.join(name);
            std::fs::write(&path, text).expect("tool");
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).expect("mode");
        }
    }
}
fn run(command: &mut Command) -> Output {
    command.output().expect("plumb should run")
}
fn identity() -> Value {
    json!({
        "repository": "PerishLab/plumb",
        "authority": AUTHORITY,
        "beta": BETA,
        "stable": STABLE
    })
}
fn beta() -> Value {
    json!({
        "schema": 1,
        "product": "plumb",
        "channel": "beta",
        "releaseVersion": BETA,
        "commit": COMMIT,
        "url": format!("{AUTHORITY}/v1/releases/beta/{BETA}/seal.json"),
        "generator": {
            "version": STABLE,
            "template": "template-digest",
            "origin": {
                "kind": "source-built",
                "repository": "PerishLab/plumb",
                "commit": SOURCE
            },
            "recovery": identity()
        },
        "artifacts": {},
        "managers": {},
        "proof": null,
        "radius": null
    })
}

fn stable(beta: Value, digest: &str) -> Value {
    json!({
        "schema": 1,
        "product": "plumb",
        "channel": "stable",
        "releaseVersion": STABLE,
        "commit": COMMIT,
        "url": format!("{AUTHORITY}/v1/releases/stable/{STABLE}/seal.json"),
        "generator": {
            "version": BETA,
            "template": "template-digest",
            "origin": {
                "kind": "exact-release",
                "channel": "beta",
                "releaseVersion": BETA,
                "url": format!("{AUTHORITY}/v1/releases/beta/{BETA}/seal.json"),
                "sha256": digest
            },
            "recovery": identity()
        },
        "artifacts": {},
        "managers": {},
        "proof": {"seal": beta, "digest": digest},
        "radius": null
    })
}

fn inspect(fixture: &Inspect<'_>, seal: &Value) -> Output {
    let url = seal["url"].as_str().expect("seal url");
    let key = url
        .strip_prefix(&format!("{AUTHORITY}/"))
        .expect("authority");
    let path = fixture.root.join("releases").join(key);
    std::fs::create_dir_all(path.parent().expect("seal parent")).expect("seal root");
    std::fs::write(&path, serde_json::to_vec(seal).expect("seal json")).expect("seal");
    run(fixture
        .command()
        .args(["release", "inspect"])
        .env("PLUMB_RELEASE_URL", url))
}

fn fixture() -> (tempfile::TempDir, std::path::PathBuf) {
    let temp = tempfile::tempdir().expect("root");
    let tools = temp.path().join("tools");
    std::fs::create_dir(&tools).expect("tools");
    (temp, tools)
}

#[test]
fn unknown_and_drifted_provenance_refuse() {
    let (temp, tools) = fixture();
    let held = Inspect {
        root: temp.path(),
        tools: &tools,
    };
    held.seed();
    let mut unknown = beta();
    unknown["generator"]["origin"]["future"] = json!(true);
    let output = inspect(&held, &unknown);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unknown field"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    for drift in ["version", "url", "repository", "identity"] {
        let mut seal = beta();
        match drift {
            "version" => seal["generator"]["version"] = json!("v0.18.13"),
            "url" => seal["url"] = json!(format!("{AUTHORITY}/v1/releases/beta/drift.json")),
            "repository" => seal["generator"]["origin"]["repository"] = json!("Other/plumb"),
            _ => seal["generator"]["recovery"]["beta"] = json!("v0.20.0-beta.2"),
        }
        assert!(!inspect(&held, &seal).status.success(), "{drift}");
    }
}

#[test]
fn stable_binds_same_exact_beta_seal() {
    let (temp, tools) = fixture();
    let held = Inspect {
        root: temp.path(),
        tools: &tools,
    };
    held.seed();
    let beta = beta();
    let bytes = serde_json::to_vec(&beta).expect("beta");
    let round: Value = serde_json::from_slice(&bytes).expect("round trip");
    assert_eq!(round, beta);
    let legacy: Value = serde_json::from_str(r#"{"version":"v0.18.13","template":"abc"}"#)
        .expect("legacy generator");
    assert_eq!(legacy, json!({"version": "v0.18.13", "template": "abc"}));
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let seal = stable(beta, &digest);
    let output = inspect(&held, &seal);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let mut drift = seal;
    drift["generator"]["origin"]["sha256"] = json!("0".repeat(64));
    assert!(!inspect(&held, &drift).status.success());
}
fn cli(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(args)
        .current_dir(root)
        .env("FORGEJO_TOKEN", "test-token")
        .output()
        .expect("plumb")
}

fn repository(root: &Path, origin: &str) {
    for args in [vec!["init", "-q"], vec!["remote", "add", "origin", origin]] {
        let status = Command::new("git")
            .args(args)
            .current_dir(root)
            .status()
            .expect("git");
        assert!(status.success());
    }
}

fn argv(caller: &str) -> [&str; 7] {
    [
        "ship",
        "binary",
        "recovery",
        "beta",
        "--caller",
        caller,
        "--dry-run",
    ]
}

fn recovery(root: &Path, authority: &str) {
    repository(root, "ssh://git@git.perish.top/PerishLab/plumb.git");
    std::fs::write(root.join("plumb.toml"), format!("[release]\nproduct = \"plumb\"\nauthority = \"{authority}\"\nbinaries = [\"plumb\"]\ntargets = [\"x86_64-unknown-linux-gnu\"]\n")).expect("manifest");
}

#[test]
fn recovery_cli_is_exact_and_typed() {
    let fixture = tempfile::tempdir().expect("fixture");
    recovery(fixture.path(), "https://releases.plumb.perish.uk");
    let caller = "cccccccccccccccccccccccccccccccccccccccc";
    let output = cli(fixture.path(), &argv(caller));
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    for expected in ["release-recovery.yml", "v0.20.0-beta.1", caller] {
        assert!(text.contains(expected), "{text}");
    }
    let output = cli(
        fixture.path(),
        &[
            "ship",
            "binary",
            "recovery",
            "beta",
            "--caller",
            caller,
            "--channel",
            "canary",
            "--dry-run",
        ],
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unexpected argument '--channel'"));
}

#[test]
fn recovery_cli_refuses_identity_and_digest_drift() {
    let caller = "cccccccccccccccccccccccccccccccccccccccc";
    let fixture = tempfile::tempdir().expect("fixture");
    recovery(fixture.path(), "https://other.invalid");
    let output = cli(fixture.path(), &argv(caller));
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("belongs only to PerishLab/plumb"));
    let exact = tempfile::tempdir().expect("fixture");
    recovery(exact.path(), "https://releases.plumb.perish.uk");
    let output = cli(
        exact.path(),
        &[
            "ship",
            "binary",
            "recovery",
            "stable",
            "--caller",
            caller,
            "--beta-sha256",
            "drift",
            "--dry-run",
        ],
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("lowercase SHA-256"));
}
