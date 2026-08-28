use super::super::world::{Fixture, run};
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

#[test]
fn exact() {
    let temp = tempfile::tempdir().expect("temp root");
    let bare = tempfile::tempdir().expect("bare root");
    let tools = temp.path().join("tools");
    std::fs::create_dir(&tools).expect("tool root");
    let fixture = Fixture {
        root: temp.path(),
        tools: &tools,
    };
    let commit = seeded(&fixture, bare.path());
    stamp(temp.path(), "v1.2.0-beta.1", &commit, true);

    let shown = marker(&fixture, "show", "v1.2.0-beta.1");
    assert!(
        shown.status.success(),
        "{}",
        String::from_utf8_lossy(&shown.stderr)
    );
    let value: Value = serde_json::from_slice(&shown.stdout).expect("marker json");
    assert_eq!(value["schema"], "plumb.release-marker/v1");
    assert_eq!(value["product"], "probe");
    assert!(
        value["repository"]
            .as_str()
            .is_some_and(|repository| repository.contains('/'))
    );
    assert_eq!(value["marker"], "v1.2.0-beta.1");
    assert_eq!(value["channel"], "beta");
    assert_eq!(value["commit"], commit);
    assert_eq!(value["state"], "locked");
    assert!(value.get("promotion").is_none());

    let verified = marker(&fixture, "verify", "1.2.0-beta.1");
    assert!(verified.status.success());
    assert!(String::from_utf8_lossy(&verified.stdout).contains("verified release marker"));
}

#[test]
fn stable() {
    let temp = tempfile::tempdir().expect("temp root");
    let bare = tempfile::tempdir().expect("bare root");
    let tools = temp.path().join("tools");
    std::fs::create_dir(&tools).expect("tool root");
    let fixture = Fixture {
        root: temp.path(),
        tools: &tools,
    };
    let commit = seeded(&fixture, bare.path());
    stamp(temp.path(), "v1.2.0-beta.1", &commit, true);
    stamp(temp.path(), "v1.2.0", &commit, true);
    seal(temp.path(), &commit);

    let shown = marker(&fixture, "show", "v1.2.0");
    assert!(
        shown.status.success(),
        "{}",
        String::from_utf8_lossy(&shown.stderr)
    );
    let value: Value = serde_json::from_slice(&shown.stdout).expect("marker json");
    assert_eq!(value["channel"], "stable");
    assert_eq!(value["promotion"]["marker"], "v1.2.0-beta.1");
    assert_eq!(value["promotion"]["channel"], "beta");
    assert_eq!(
        value["promotion"]["seal"]["url"],
        "https://releases.test/v1/releases/beta/v1.2.0-beta.1/seal.json"
    );
}

#[test]
fn refusal() {
    let temp = tempfile::tempdir().expect("temp root");
    let bare = tempfile::tempdir().expect("bare root");
    let tools = temp.path().join("tools");
    std::fs::create_dir(&tools).expect("tool root");
    let fixture = Fixture {
        root: temp.path(),
        tools: &tools,
    };
    let commit = seeded(&fixture, bare.path());
    stamp(temp.path(), "v1.2.0-beta.2", &commit, false);
    let output = marker(&fixture, "verify", "v1.2.0-beta.2");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("is not an annotated tag"));

    let stable = fixture
        .command()
        .current_dir(fixture.root)
        .args(["release", "stamp", "--version", "v1.2.0", "--dry-run"])
        .output()
        .expect("plumb should run");
    assert!(!stable.status.success());
    assert!(
        String::from_utf8_lossy(&stable.stderr)
            .contains("stable marker v1.2.0 is created by plumb release freeze")
    );
}

fn marker(fixture: &Fixture<'_>, deed: &str, name: &str) -> Output {
    fixture
        .command()
        .current_dir(fixture.root)
        .args(["release", "marker", deed, "--marker", name])
        .output()
        .expect("plumb should run")
}

fn seeded(fixture: &Fixture<'_>, bare: &Path) -> String {
    fixture.seed();
    std::fs::create_dir_all(fixture.root.join(".plumb/releases/v1.2.0")).expect("datum root");
    std::fs::write(
        fixture.root.join(".plumb/releases/v1.2.0/datum.toml"),
        "schema = 1\nversion = \"v1.2.0\"\n",
    )
    .expect("datum");
    run(Command::new("git")
        .arg("-C")
        .arg(fixture.root)
        .args(["add", "plumb.toml", ".plumb"]));
    run(Command::new("git")
        .arg("-C")
        .arg(fixture.root)
        .args(["commit", "-qm", "candidate"]));
    run(Command::new("git").args(["init", "-q", "--bare"]).arg(bare));
    run(Command::new("git").arg("-C").arg(fixture.root).args([
        "remote",
        "add",
        "origin",
        &format!("file://{}", bare.display()),
    ]));
    run(Command::new("git").arg("-C").arg(fixture.root).args([
        "push",
        "-q",
        "origin",
        "HEAD:refs/heads/main",
    ]));
    run(Command::new("git").arg("-C").arg(fixture.root).args([
        "push",
        "-q",
        "origin",
        "HEAD:refs/heads/release/v1.2.0",
    ]));
    String::from_utf8(
        run(Command::new("git")
            .arg("-C")
            .arg(fixture.root)
            .args(["rev-parse", "HEAD"]))
        .stdout,
    )
    .expect("commit utf8")
    .trim()
    .to_string()
}

fn stamp(root: &Path, version: &str, commit: &str, annotated: bool) {
    let mut command = Command::new("git");
    command.arg("-C").arg(root).arg("tag");
    if annotated {
        command.args(["-a", version, commit, "-m", &format!("probe {version}")]);
    } else {
        command.args([version, commit]);
    }
    run(&mut command);
    run(Command::new("git").arg("-C").arg(root).args([
        "push",
        "-q",
        "origin",
        &format!("refs/tags/{version}"),
    ]));
}

fn seal(root: &Path, commit: &str) {
    let version = "v1.2.0-beta.1";
    let path = root
        .join("releases/v1/releases/beta")
        .join(version)
        .join("seal.json");
    std::fs::create_dir_all(path.parent().expect("seal parent")).expect("seal root");
    let value = serde_json::json!({
        "schema": 1,
        "product": "probe",
        "channel": "beta",
        "releaseVersion": version,
        "commit": commit,
        "url": format!("https://releases.test/v1/releases/beta/{version}/seal.json"),
        "generator": { "version": "v0.37.6", "template": "0" },
        "artifacts": {},
        "managers": {}
    });
    std::fs::write(
        path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&value).expect("seal json")
        ),
    )
    .expect("seal");
}
