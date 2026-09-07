use super::super::world::{Fixture, run};
use serde_json::Value;
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;
use std::process::Command;

pub(super) fn candidates(
    fixture: &Fixture<'_>,
    command: &impl Fn() -> Command,
    annotation: &Value,
) {
    let mut beta = annotation.clone();
    beta["marker"] = serde_json::json!("v1.2.0-beta.1");
    run(Command::new("git").arg("-C").arg(fixture.root).args([
        "tag",
        "-a",
        "v1.2.0-beta.1",
        "-m",
        &beta.to_string(),
    ]));
    let output = run(command().args([
        "ship",
        "resolve",
        "--marker",
        "v1.2.0-beta.1",
        "--atom",
        &"a".repeat(40),
    ]));
    let graph: Value = serde_json::from_slice(&output.stdout).expect("candidate graph");
    assert_eq!(graph["workload_missing"], false);
    assert_eq!(graph["publication_missing"], true);
    let records = graph["publication"]["include"]
        .as_array()
        .expect("requests")
        .iter()
        .map(|row| {
            let request = &row["request"];
            let keys = &request["keys"];
            serde_json::json!({
                "action": request["action"], "workload": keys["workload"],
                "proof": keys["proof"], "publication": keys["publication"],
                "source": { "type": "url", "source": "https://registry.test/probe" },
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(records.len(), 2);
    std::fs::create_dir(fixture.root.join("depot")).expect("inventory root");
    let mut cases = (0..=records.len())
        .map(|count| (records[..count].to_vec(), count == records.len()))
        .collect::<Vec<_>>();
    for field in ["workload", "publication"] {
        let mut drift = records.clone();
        drift[0][field] = serde_json::json!("0".repeat(64));
        cases.push((drift, false));
    }
    for (records, complete) in cases {
        let inventory = serde_json::json!({
            "schema": "plumb.workflow-inventory/v1",
            "records": records,
        });
        std::fs::write(
            fixture.root.join("depot/inventory.json"),
            inventory.to_string(),
        )
        .expect("inventory");
        let stable = command()
            .args([
                "ship",
                "resolve",
                "--marker",
                "v1.2.0",
                "--atom",
                &"a".repeat(40),
            ])
            .output()
            .expect("stable graph");
        assert_eq!(
            stable.status.success(),
            complete,
            "{}",
            String::from_utf8_lossy(&stable.stderr)
        );
    }
}

#[test]
fn retained() {
    let temp = tempfile::tempdir().expect("temp root");
    let bare = tempfile::tempdir().expect("bare root");
    let home = tempfile::tempdir().expect("plumb home");
    let tools = temp.path().join("tools");
    std::fs::create_dir(&tools).expect("tool root");
    let fixture = Fixture {
        root: temp.path(),
        tools: &tools,
    };
    let commit = super::marker::seeded(&fixture, bare.path());
    let (first, profile) = configuration(home.path(), "old", None);
    let (latest, _) = configuration(home.path(), "new", Some(&first));
    assert_ne!(first, latest);
    let version = plumb::version!("PLUMB").to_string();
    let annotation = serde_json::json!({
        "schema": "plumb.release-marker/v2",
        "product": "probe",
        "marker": "v1.2.0-beta.1",
        "configuration": {
            "channel": "stable",
            "version": version,
            "generation": first,
        },
        "profile": profile,
    });
    annotate(
        fixture.root,
        "v1.2.0-beta.1",
        &commit,
        &annotation.to_string(),
    );
    run(Command::new("git").arg("-C").arg(fixture.root).args([
        "remote",
        "set-url",
        "origin",
        "ssh://git@git.perish.top/PerishFire/probe.git",
    ]));
    let shown = fixture
        .command()
        .current_dir(fixture.root)
        .env("PLUMB_HOME", home.path())
        .env("PLUMB_RULES_SOURCE", "https://depot.test")
        .args(["release", "show", "--marker", "v1.2.0-beta.1", "--held"])
        .output()
        .expect("plumb should run");
    assert!(
        shown.status.success(),
        "{}",
        String::from_utf8_lossy(&shown.stderr)
    );
    let value: Value = serde_json::from_slice(&shown.stdout).expect("marker json");
    assert_eq!(value["schema"], "plumb.release-marker/v2");
    assert_eq!(value["authority"], "https://releases.old.perish.uk");
    assert_eq!(value["configuration"], first);
    assert_eq!(value["profile"], profile);

    let git = tools.join("git");
    std::fs::write(
        &git,
        "#!/bin/sh\nfor arg in \"$@\"; do [ \"$arg\" = fetch ] && exit 0; done\nexec /usr/bin/git \"$@\"\n",
    )
    .expect("git shim");
    std::fs::set_permissions(&git, std::fs::Permissions::from_mode(0o755)).expect("git shim mode");
    let skill = temp.path().join("skill");
    std::fs::create_dir(&skill).expect("skill root");
    std::fs::write(skill.join("SKILL.md"), "# Probe\n").expect("skill body");
    let depot = fixture
        .command()
        .current_dir(fixture.root)
        .env("PLUMB_HOME", home.path())
        .env("PLUMB_RULES_SOURCE", "https://depot.test")
        .args([
            "depot",
            "skill",
            ".",
            "--marker",
            "v1.2.0-beta.1",
            "--from",
            skill.to_str().expect("skill path"),
            "--dry-run",
        ])
        .output()
        .expect("profile depot should run");
    assert!(
        depot.status.success(),
        "{}",
        String::from_utf8_lossy(&depot.stderr)
    );
}

fn annotate(root: &Path, version: &str, commit: &str, message: &str) {
    run(Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["tag", "-a", version, commit, "-m", message]));
    run(Command::new("git").arg("-C").arg(root).args([
        "push",
        "-q",
        "origin",
        &format!("refs/tags/{version}"),
    ]));
}

pub(super) fn configuration(home: &Path, authority: &str, prior: Option<&str>) -> (String, String) {
    let source = tempfile::tempdir().expect("configuration source");
    let profile = format!(
        "schema = \"plumb.product-profile/v1\"\n\n[product]\nname = \"probe\"\nauthority = \"https://releases.{authority}.perish.uk\"\nderivatives = [\"skill\"]\n\n[governance]\nmanifest = '''\n[release]\nproduct = \"probe\"\nauthority = \"https://releases.{authority}.perish.uk\"\n[release.cargo]\nregistry = \"perish\"\npackages = [\"probe\"]\n[release.oci]\nregistry = \"registry.test\"\nimage = \"owner/probe\"\naccount = \"probe\"\n'''\nectropy = \"[comment]\\nallow = false\"\n"
    );
    let digest = plumb::depot::sha(profile.as_bytes());
    let catalog = format!(
        "schema = \"plumb.products/v2\"\n\n[[product]]\nidentity = \"git.perish.top/PerishFire/probe\"\nprofile = \"{digest}\"\n"
    );
    let address = format!("profiles/{digest}.toml");
    crate::support::stock(
        &source.path().join("configurations"),
        &[("rules/products.toml", &catalog), (&address, &profile)],
    );
    let version = plumb::version!("PLUMB").to_string();
    let bundle = plumb::depot::v3::Bundle::read(
        &source
            .path()
            .join("configurations")
            .join("29990101T000000Z"),
        plumb::depot::v3::Identity {
            product: "plumb".into(),
            channel: "stable".into(),
            version: version.clone(),
            marker: plumb::depot::v3::Marker {
                name: version,
                sha256: "a".repeat(64),
            },
            kind: plumb::depot::v3::Kind::Configuration,
        },
    )
    .expect("configuration bundle");
    let pointer = plumb::depot::v3::Pointer::new(
        &bundle.manifest,
        plumb::depot::v3::Publication {
            source: "https://depot.test",
            prior: prior.map(str::to_string),
            created: "2026-09-03T01:02:03Z".into(),
        },
    )
    .expect("configuration pointer");
    plumb::depot::v3::install(&home.join("configurations"), &pointer, &bundle)
        .expect("install configuration");
    (pointer.generation, digest)
}
