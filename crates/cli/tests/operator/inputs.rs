use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;

pub fn workload(command: &mut Command, archive: &Path, targets: usize) -> Value {
    let graph = graph(command, "v1.2.0-beta.7");
    assert_eq!(
        graph["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|node| node["execution"]["payload"]["operation"]["type"] == "bind")
            .count(),
        targets
    );
    let request = &node(&graph, "ship/binary.x86_64-unknown-linux-gnu")["execution"]["payload"];
    assert_eq!(request["operation"]["type"], "bind");
    let tools = json!({"cargo":{"path":"/fixture/cargo","digest":"c".repeat(64)}});
    json!({
        "target": request["operation"]["target"],
        "archive": request["operation"]["archive"],
        "url": "https://inventory.invalid/binary.tar.gz",
        "receipt": {
            "schema": "plumb.production-receipt/v1",
            "contract": request["production"],
            "platform": "linux-x86_64",
            "artifact": plumb::depot::sha(&std::fs::read(archive).unwrap()),
            "environment": "b".repeat(64),
            "tools": tools,
            "observations": {"rule://seat/compiler":"fixture cargo"},
        },
    })
}

fn graph(command: &mut Command, marker: &str) -> Value {
    let output = command
        .args([
            "ship",
            "resolve",
            "--marker",
            marker,
            "--atom",
            &"a".repeat(40),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let graph: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(graph["schema"], "plumb.blob-graph/v1");
    for node in graph["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|node| node["id"].as_str().unwrap().starts_with("prepare/"))
    {
        assert_eq!(node["execution"]["payload"]["commit"], "a".repeat(40));
    }
    graph
}

fn node<'a>(graph: &'a Value, name: &str) -> &'a Value {
    graph["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["id"] == name)
        .unwrap()
}

#[test]
fn binding() {
    let root = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let tools = root.path().join("tools");
    std::fs::create_dir(&tools).unwrap();
    let fixture = crate::world::Fixture {
        root: root.path(),
        tools: &tools,
    };
    fixture.seed();
    std::fs::write(
        root.path().join("Cargo.toml"),
        "[workspace]\n[workspace.package]\nversion='1.2.0'\n",
    )
    .unwrap();
    crate::marker::prepare(
        root.path(),
        home.path(),
        "[[layout.file]]\nname=['Cargo.toml']\nrule=['rule://seat/compiler']\n[release]\nproduct='probe'\nauthority='https://releases.test'\nbinaries=['probe']\ntargets=['x86_64-unknown-linux-gnu']\n",
        "v1.2.0-beta.7",
    );
    let annotation = Command::new("git")
        .arg("-C")
        .arg(root.path())
        .args([
            "for-each-ref",
            "--format=%(contents)",
            "refs/tags/v1.2.0-beta.7",
        ])
        .output()
        .unwrap();
    assert!(annotation.status.success());
    let mut annotation: Value = serde_json::from_slice(&annotation.stdout).unwrap();
    annotation["marker"] = json!("v1.2.0-beta.8");
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(root.path())
            .args(["tag", "-a", "v1.2.0-beta.8", "-m", &annotation.to_string()])
            .status()
            .unwrap()
            .success()
    );
    let command = || {
        let mut held = fixture.command();
        held.current_dir(root.path())
            .env("PLUMB_HOME", home.path())
            .env("PLUMB_RULES_SOURCE", "https://depot.test")
            .env_remove("PLUMB_RELEASE_CHANNEL")
            .env_remove("PLUMB_RELEASE_COMMIT");
        held
    };
    let first = graph(&mut command(), "v1.2.0-beta.7");
    let next = graph(&mut command(), "v1.2.0-beta.8");
    let produce = "ship/produce.x86_64-unknown-linux-gnu";
    let bind = "ship/binary.x86_64-unknown-linux-gnu";
    assert_eq!(
        node(&first, produce)["inputs"],
        node(&next, produce)["inputs"]
    );
    assert_ne!(
        node(&first, bind)["inputs"]["identity"],
        node(&next, bind)["inputs"]["identity"]
    );
    assert_eq!(
        node(&first, bind)["inputs"]["content"],
        node(&next, bind)["inputs"]["content"]
    );
    let first = &node(&first, bind)["execution"]["payload"];
    let content = &first["operation"]["build"];
    assert_ne!(content["production"], first["production"]);
    for (field, expected) in [
        ("keys", "unknown field"),
        ("production", "reusable production contract differs"),
    ] {
        let mut drift = first.clone();
        drift["operation"]["build"][field] = json!("wrong");
        let output = command()
            .env("PLUMB_RELEASE_VERSION", "v1.2.0-beta.7")
            .args(["ship", "execute", "--request", &drift.to_string()])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let mut missing = first.clone();
    missing["operation"]["build"]["reuse"] = json!({"type":"workload","source":format!("https://inventory.invalid/{}.tgz", "a".repeat(64))});
    let output = command()
        .env("PLUMB_RELEASE_VERSION", "v1.2.0-beta.7")
        .args(["ship", "execute", "--request", &missing.to_string()])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("no production receipt"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

pub fn refuses(command: &impl Fn(&Value) -> Command, request: &Value, root: &Path) {
    let original = request["operation"]["workloads"][0].clone();
    let mut cases = vec![
        (json!([]), "no proven binary workload"),
        (
            json!([original, original]),
            "duplicate binary workload target",
        ),
    ];
    for (field, value, expected) in [
        (
            "archive",
            json!("wrong.tar.gz"),
            "archive differs from its target",
        ),
        ("target", json!("undeclared"), "target"),
        ("url", json!("file:///tmp/archive"), "HTTPS URL"),
        ("receipt", Value::Null, "invalid type"),
    ] {
        let mut held = original.clone();
        held[field] = value;
        cases.push((json!([held]), expected));
    }
    let mut drift = original.clone();
    drift["receipt"]["contract"] = json!("0".repeat(64));
    cases.push((json!([drift]), "does not bind this contract"));
    for (workloads, expected) in cases {
        let mut request = request.clone();
        request["operation"]["workloads"] = workloads;
        let output = command(&request).output().unwrap();
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            !output.status.success() && error.contains(expected),
            "{error}"
        );
        assert!(!root.join("fetched").exists());
        assert!(!root.join("perish-probe-releases").exists());
    }
    let mut corrupt = request.clone();
    corrupt["operation"]["workloads"][0]["receipt"]["artifact"] = json!("0".repeat(64));
    let output = command(&corrupt).output().unwrap();
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success() && error.contains("artifact differs from its receipt"),
        "{error}"
    );
    assert!(root.join("fetched").exists());
    assert!(!root.join("perish-probe-releases").exists());
}

#[test]
fn partial() {
    let root = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let tools = root.path().join("tools");
    std::fs::create_dir_all(&tools).unwrap();
    let fixture = crate::world::Fixture {
        root: root.path(),
        tools: &tools,
    };
    fixture.seed();
    std::fs::write(
        root.path().join("Cargo.toml"),
        "[workspace]\n[workspace.package]\nversion='1.2.0'\n",
    )
    .unwrap();
    let manifest = "[[layout.file]]\nname=['Cargo.toml']\nrule=['rule://seat/compiler']\n[release]\nproduct='probe'\nauthority='https://releases.test'\nbinaries=['probe']\ntargets=['x86_64-unknown-linux-gnu','aarch64-apple-darwin']\n";
    let binding = crate::marker::prepare(root.path(), home.path(), manifest, "v1.2.0-beta.7");
    let command = || {
        let mut command = fixture.command();
        command
            .current_dir(root.path())
            .env("PLUMB_HOME", home.path())
            .env("PLUMB_RULES_SOURCE", "https://depot.test")
            .env("PLUMB_RELEASE_VERSION", "v1.2.0-beta.7")
            .env_remove("PLUMB_RELEASE_CHANNEL")
            .env_remove("PLUMB_RELEASE_COMMIT");
        command
    };
    let artifact = root.path().join("binary.tar");
    std::fs::write(&artifact, "untrusted archive").unwrap();
    let workload = workload(&mut command(), &artifact, 2);
    let request = json!({
        "schema":"plumb.ship-request/v3", "marker":"v1.2.0-beta.7", "configuration":binding["configuration"], "profile":binding["profile"],
        "action":"ship/binary",
        "operation":{"type":"publication","workloads":[workload]},
    });
    let output = command()
        .args(["ship", "execute", "--request", &request.to_string()])
        .env("PLUMB_PUBLISH_ENDPOINT", "https://s3.test")
        .env(
            "PLUMB_PUBLISH_FINGERPRINT",
            plumb::depot::sha(b"https://s3.test"),
        )
        .output()
        .unwrap();
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success()
            && error.contains("no proven binary workload for aarch64-apple-darwin"),
        "{error}"
    );
    assert!(!root.path().join("dist/v1.2.0-beta.7").exists());
    assert!(!root.path().join("perish-probe-releases").exists());
}
