use super::super::world::Fixture;
use serde_json::{Value, json};
use std::process::Command;

pub(crate) fn identity(
    fixture: &Fixture<'_>,
    command: &impl Fn() -> Command,
    graph: &Value,
    annotation: &Value,
) {
    let mut marker = annotation.clone();
    marker["marker"] = json!("v1.2.0-beta.2");
    super::super::world::run(Command::new("git").arg("-C").arg(fixture.root).args([
        "tag",
        "-a",
        "v1.2.0-beta.2",
        "-m",
        &marker.to_string(),
    ]));
    let output = super::super::world::run(command().args([
        "ship",
        "resolve",
        "--marker",
        "v1.2.0-beta.2",
        "--atom",
        &"a".repeat(40),
    ]));
    let current: Value = serde_json::from_slice(&output.stdout).unwrap();
    let image = |graph: &Value| {
        graph["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["id"] == "ship/oci")
            .unwrap()
            .clone()
    };
    let prior = image(graph);
    let current = image(&current);
    for field in ["source", "implementation", "production"] {
        assert_eq!(prior["inputs"][field], current["inputs"][field], "{field}");
    }
    assert_ne!(prior["inputs"]["identity"], current["inputs"]["identity"]);
    assert_eq!(
        prior["execution"]["payload"]["production"],
        current["execution"]["payload"]["production"]
    );
    super::super::world::run(Command::new("git").arg("-C").arg(fixture.root).args([
        "tag",
        "-d",
        "v1.2.0-beta.2",
    ]));
}

pub(crate) fn refuses(fixture: &Fixture<'_>, command: &impl Fn() -> Command, graph: &Value) {
    let mut request = graph["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| &row["execution"]["payload"])
        .find(|request| request["action"] == "ship/oci")
        .unwrap()
        .clone();
    let archive = fixture.root.join("held.tar");
    std::fs::write(&archive, "wrong archive").unwrap();
    request["reuse"] = json!({"type":"workload","source":format!("https://inventory.test/v2/blobs/sha256/{}", plumb::depot::sha(b"wrong archive"))});
    let original = std::fs::read_to_string(fixture.tools.join("curl")).unwrap();
    let curl = original.replace(
        "case \"$url\" in",
        "case \"$url\" in\n  https://inventory.test/v2/blobs/sha256/*) cat \"$FAKE_S3_ROOT/held.tar\"; exit 0 ;;",
    );
    std::fs::write(fixture.tools.join("curl"), curl).unwrap();
    for program in ["docker", "regctl"] {
        super::executable(
            &fixture.tools.join(program),
            "#!/bin/sh\necho unexpected-tool >&2\nexit 97\n",
        );
    }
    let held = receipt(&request);
    request["operation"]["workloads"] = json!([{
        "target":"must-not-materialize", "archive":"absent.tar.gz",
        "url":"https://inventory.test/forbidden-download", "receipt":held,
    }]);
    for (receipt, expected) in [
        (Value::Null, "no production receipt"),
        (held.clone(), "artifact differs from its receipt"),
    ] {
        request["receipt"] = receipt;
        let output = command()
            .args(["ship", "execute", "--request", &request.to_string()])
            .env("PLUMB_RELEASE_VERSION", "v1.2.0-beta.1")
            .output()
            .unwrap();
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            !output.status.success() && error.contains(expected),
            "{error}"
        );
        assert!(!error.contains("unexpected-tool"), "{error}");
    }
    super::container::archive(&archive, 1);
    let mut receipt = held;
    receipt["artifact"] = json!(plumb::depot::sha(&std::fs::read(&archive).unwrap()));
    request["reuse"]["source"] = json!(format!(
        "https://inventory.test/v2/blobs/sha256/{}",
        receipt["artifact"].as_str().unwrap()
    ));
    request["receipt"] = receipt;
    let scenario = fixture.root.join("scenario");
    let calls = fixture.root.join("calls");
    std::fs::write(&scenario, "login").unwrap();
    std::fs::write(&calls, "").unwrap();
    let client = super::registry::CLIENT.replace("set -eu", &format!(
        "set -eu\n[ \"$1\" != version ] || {{ printf 'fixture regctl'; exit 0; }}\nPLUMB_TEST_SCENARIO='{}'\nPLUMB_TEST_CALLS='{}'",
        scenario.display(), calls.display(),
    )).replace("registry.example", "registry.test").replace("Example", "probe");
    super::executable(&fixture.tools.join("regctl"), &client);
    let output = command()
        .args(["ship", "execute", "--request", &request.to_string()])
        .env("PLUMB_RELEASE_VERSION", "v1.2.0-beta.1")
        .env("PLUMB_RELEASE_REGISTRY_TOKEN", "Bearer secret")
        .output()
        .unwrap();
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success() && error.contains("login failed"),
        "{error}"
    );
    let calls = std::fs::read_to_string(calls).unwrap();
    assert!(calls.contains("image import"), "{calls}");
    assert!(!calls.contains("image copy"), "{calls}");
    assert!(!error.contains("unexpected-tool"), "{error}");
    std::fs::write(fixture.tools.join("curl"), original).unwrap();
}

fn receipt(request: &Value) -> Value {
    json!({
        "schema":"plumb.production-receipt/v1", "contract":request["production"],
        "platform":"linux-x86_64", "artifact":"a".repeat(64), "environment":"b".repeat(64),
        "tools": {"docker": {"path":"/absent/docker","digest":"c".repeat(64)}},
        "observations": {"rule://seat/docker":"fixture docker"},
    })
}
