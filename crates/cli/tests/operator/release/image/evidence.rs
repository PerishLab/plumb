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
        graph["publication"]["include"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| &row["request"])
            .find(|request| request["action"] == "ship/oci")
            .unwrap()
            .clone()
    };
    let prior = image(graph);
    let current = image(&current);
    for field in ["workload", "proof"] {
        assert_eq!(prior["keys"][field], current["keys"][field], "{field}");
    }
    assert_eq!(prior["production"], current["production"]);
    assert_ne!(prior["keys"]["publication"], current["keys"]["publication"]);
    let held = receipt(&prior);
    let mut record = json!({
        "action":"ship/oci", "workload":prior["keys"]["workload"],
        "proof":prior["keys"]["proof"],
        "receipt":held, "source":{"type":"workload",
            "source":format!("https://inventory.test/workloads/{}.tgz", "a".repeat(64))},
    });
    std::fs::create_dir_all(fixture.root.join("depot")).unwrap();
    for kind in ["workload", "none"] {
        std::fs::write(
            fixture.root.join("depot/inventory.json"),
            json!({
                "schema":"plumb.workflow-inventory/v1", "records":[record],
            })
            .to_string(),
        )
        .unwrap();
        let output = super::super::world::run(command().args([
            "ship",
            "resolve",
            "--marker",
            "v1.2.0-beta.2",
            "--atom",
            &"a".repeat(40),
        ]));
        let graph: Value = serde_json::from_slice(&output.stdout).unwrap();
        let request = image(&graph);
        assert_eq!(request["reuse"]["type"], kind);
        assert_eq!(request["receipt"].is_null(), kind == "none");
        assert_eq!(
            request["operation"]["workloads"].is_null(),
            kind == "workload"
        );
        record["proof"] = json!("0".repeat(64));
    }
    super::super::world::run(Command::new("git").arg("-C").arg(fixture.root).args([
        "tag",
        "-d",
        "v1.2.0-beta.2",
    ]));
}

pub(crate) fn refuses(fixture: &Fixture<'_>, command: &impl Fn() -> Command, graph: &Value) {
    let mut request = graph["publication"]["include"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| &row["request"])
        .find(|request| request["action"] == "ship/oci")
        .unwrap()
        .clone();
    request["reuse"] = json!({"type":"workload","source":"https://inventory.test/reused.tgz"});
    let original = std::fs::read_to_string(fixture.tools.join("curl")).unwrap();
    let curl = original.replace(
        "case \"$url\" in",
        "case \"$url\" in\n  https://inventory.test/reused.tgz) printf 'wrong archive'; exit 0 ;;",
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
    let archive = fixture.root.join("held.tar");
    super::container::archive(&archive, 1);
    let mut receipt = held;
    receipt["artifact"] = json!(plumb::depot::sha(&std::fs::read(&archive).unwrap()));
    request["receipt"] = receipt;
    std::fs::write(fixture.tools.join("curl"), original.replace(
        "case \"$url\" in",
        "case \"$url\" in\n  https://inventory.test/reused.tgz) cat \"$FAKE_S3_ROOT/held.tar\"; exit 0 ;;",
    )).unwrap();
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
