use super::super::world::{Fixture, run};
use super::marker::{seeded, stamp};
use super::profile::configuration;
use serde_json::Value;
use std::os::unix::fs::PermissionsExt as _;
use std::process::Command;

pub(super) fn bound(command: &impl Fn() -> Command, graph: &Value, store: &crate::support::Bucket) {
    use sha2::{Digest, Sha256};
    let output = run(command().args(["release", "verify", "--marker", "v1.2.0-beta.1", "--held"]));
    let output = String::from_utf8(output.stdout).unwrap();
    let marker = output
        .rsplit_once('(')
        .unwrap()
        .1
        .trim()
        .trim_end_matches(')');
    let key = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(marker, "oci://registry.test/owner/probe")).unwrap())
    );
    let request = graph["publication"]["include"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| &row["request"])
        .find(|request| request["action"] == "ship/oci")
        .unwrap();
    let source = format!(
        "https://registry.test/v2/owner/probe/manifests/sha256:{}",
        "a".repeat(64)
    );
    let mut record = serde_json::json!({
        "action": "ship/oci", "workload": "0".repeat(64), "proof": "0".repeat(64),
        "publication": "0".repeat(64), "binding": key,
        "source": { "type": "url", "source": source },
    });
    let route = format!("records/binding/{key}.json");
    store.seed(&route, &serde_json::to_vec(&record).unwrap());
    let resolved = run(command().args([
        "ship",
        "resolve",
        "--marker",
        "v1.2.0-beta.1",
        "--atom",
        &"b".repeat(40),
    ]));
    let resolved: Value = serde_json::from_slice(&resolved.stdout).unwrap();
    let rows = resolved["publication"]["include"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["request"]["action"], "ship/cargo");
    let execute = || {
        command()
            .args(["ship", "execute", "--request", &request.to_string()])
            .env("PLUMB_RELEASE_VERSION", "v1.2.0-beta.1")
            .output()
            .unwrap()
    };
    let output = execute();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let returned: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(returned["result"]["source"], source);
    record["source"]["source"] = serde_json::json!("https://registry.test/wrong-resource");
    store.seed(&route, &serde_json::to_vec(&record).unwrap());
    let refused = execute();
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("different resource"));
}

#[test]
fn independent() {
    let temp = tempfile::tempdir().expect("temp root");
    let bare = tempfile::tempdir().expect("bare root");
    let home = tempfile::tempdir().expect("plumb home");
    let tools = temp.path().join("tools");
    std::fs::create_dir(&tools).expect("tool root");
    let fixture = Fixture {
        root: temp.path(),
        tools: &tools,
    };
    super::marker::seeded(&fixture, bare.path());
    std::fs::write(
        tools.join("curl"),
        super::super::world::CURL
            .replace("https://releases.test/", "https://releases.new.perish.uk/"),
    )
    .expect("release authority fixture");
    let (generation, profile) = configuration(home.path(), "new", None);
    run(Command::new("git").arg("-C").arg(fixture.root).args([
        "remote",
        "set-url",
        "origin",
        "ssh://git@git.perish.top/PerishFire/probe.git",
    ]));
    let tree = run(Command::new("git")
        .arg("-C")
        .arg(fixture.root)
        .args(["rev-parse", "HEAD^{tree}"]));
    let proof = super::datum::proof(fixture.root, String::from_utf8_lossy(&tree.stdout).trim());
    run(Command::new("git").arg("-C").arg(fixture.root).args([
        "commit",
        "--allow-empty",
        "-qm",
        &format!("candidate\n\nPlumb-Guard-Proof: {proof}"),
    ]));
    run(Command::new("git").arg("-C").arg(fixture.root).args([
        "update-ref",
        "refs/remotes/origin/release/v1.2.0",
        "HEAD",
    ]));
    let mut annotation = serde_json::json!({
        "schema": "plumb.release-marker/v3", "product": "probe", "marker": "v1.2.0",
        "configuration": { "channel": "stable", "version": plumb::version!("PLUMB").to_string(), "generation": generation },
        "profile": profile,
    });
    run(Command::new("git").arg("-C").arg(fixture.root).args([
        "tag",
        "-a",
        "v1.2.0",
        "-m",
        &annotation.to_string(),
    ]));
    let command = || {
        let mut held = fixture.command();
        held.current_dir(fixture.root)
            .env("PLUMB_HOME", home.path())
            .env(
                "PLUMB_WORKFLOW_INVENTORY_URL",
                "https://depot.test/inventory.json",
            )
            .env("PLUMB_RULES_SOURCE", "https://depot.test");
        held
    };
    let shown = command()
        .args(["release", "show", "--marker", "v1.2.0", "--held"])
        .output()
        .expect("marker");
    assert!(
        shown.status.success(),
        "{}",
        String::from_utf8_lossy(&shown.stderr)
    );
    let value: Value = serde_json::from_slice(&shown.stdout).expect("marker json");
    assert_eq!(value["schema"], "plumb.release-marker/v3");
    assert!(value.get("promotion").is_none());
    let git = tools.join("git");
    std::fs::write(&git, "#!/bin/sh\nfor arg in \"$@\"; do [ \"$arg\" = fetch ] && exit 0; done\nexec /usr/bin/git \"$@\"\n").expect("git shim");
    std::fs::set_permissions(&git, std::fs::Permissions::from_mode(0o755)).expect("git mode");
    let skill = temp.path().join("skill");
    std::fs::create_dir(&skill).expect("skill root");
    std::fs::write(skill.join("SKILL.md"), "# Probe\n").expect("skill body");
    let depot = command()
        .args([
            "depot",
            "skill",
            ".",
            "--marker",
            "v1.2.0",
            "--from",
            skill.to_str().expect("skill path"),
            "--dry-run",
        ])
        .output()
        .expect("depot");
    assert!(
        depot.status.success(),
        "{}",
        String::from_utf8_lossy(&depot.stderr)
    );
    let ship = command()
        .args([
            "ship",
            "resolve",
            "--marker",
            "v1.2.0",
            "--atom",
            &"a".repeat(40),
        ])
        .output()
        .expect("ship");
    assert!(!ship.status.success());
    assert!(
        String::from_utf8_lossy(&ship.stderr).contains("fully proven candidate publication graph")
    );
    super::profile::candidates(&fixture, &command, &annotation);
    annotation["schema"] = serde_json::json!("plumb.release-marker/v2");
    run(Command::new("git")
        .arg("-C")
        .arg(fixture.root)
        .args(["tag", "-d", "v1.2.0"]));
    run(Command::new("git").arg("-C").arg(fixture.root).args([
        "tag",
        "-a",
        "v1.2.0",
        "-m",
        &annotation.to_string(),
    ]));
    let legacy = command()
        .args(["release", "verify", "--marker", "v1.2.0", "--held"])
        .output()
        .expect("legacy marker");
    assert!(!legacy.status.success());
    assert!(String::from_utf8_lossy(&legacy.stderr).contains("no published exact seal"));
}

#[test]
fn snapshot() {
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
    run(Command::new("git").arg("-C").arg(fixture.root).args([
        "remote",
        "set-url",
        "origin",
        "file:///missing/PerishLab/probe.git",
    ]));
    run(Command::new("git").arg("-C").arg(fixture.root).args([
        "update-ref",
        "-d",
        "refs/remotes/origin/release/v1.2.0",
    ]));

    let held = fixture
        .command()
        .current_dir(fixture.root)
        .args(["release", "show", "--marker", "v1.2.0-beta.1", "--held"])
        .output()
        .expect("plumb should run");
    assert!(
        held.status.success(),
        "a held snapshot does not consult its remote: {}",
        String::from_utf8_lossy(&held.stderr)
    );
    let refreshed = fixture
        .command()
        .current_dir(fixture.root)
        .args(["release", "show", "--marker", "v1.2.0-beta.1"])
        .output()
        .expect("plumb should run");
    assert!(!refreshed.status.success());
}
