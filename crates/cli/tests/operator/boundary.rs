use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;

fn request() -> Value {
    json!({
        "schema":"plumb.ship-request/v2", "action":"ship/chart",
        "projections":["charts/probe/Chart.yaml#/version","charts/probe/Chart.yaml#/appVersion"],
        "roots":["charts/probe"], "operation":{"type":"chart"},
        "configuration":"a".repeat(64), "profile":"b".repeat(64),
        "keys":{"workload":"1".repeat(64),"proof":"2".repeat(64),"publication":"3".repeat(64)},
    })
}

fn command(root: &Path, home: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_plumb"));
    command
        .current_dir(root)
        .env("PLUMB_HOME", home)
        .env("PLUMB_RULES_SOURCE", "https://depot.test")
        .env("PLUMB_RELEASE_ROOT", root)
        .env_remove("PLUMB_RELEASE_VERSION")
        .env_remove("PLUMB_RELEASE_CHANNEL")
        .env_remove("PLUMB_RELEASE_COMMIT");
    command
}

fn refused(command: &mut Command, request: &Value, expected: &str) {
    let output = command
        .args(["ship", "execute", "--request", &request.to_string()])
        .output()
        .unwrap();
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success() && error.contains(expected),
        "{error}"
    );
}

#[test]
fn required() {
    let root = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    for operation in [
        json!({"type":"cargo"}),
        json!({"type":"cfworker"}),
        json!({"type":"chart"}),
        json!({"type":"npm","package":"probe"}),
        json!({"type":"oci"}),
        json!({"type":"bind","target":"target","archive":"archive","build":{"reuse":{"type":"none","source":""},"keys":{},"production":"a".repeat(64),"receipt":null}}),
        json!({"type":"publication","workloads":[]}),
    ] {
        let mut original = request();
        original["operation"] = operation;
        for field in ["configuration", "profile", "keys"] {
            let mut missing = original.clone();
            missing.as_object_mut().unwrap().remove(field);
            refused(
                &mut command(root.path(), home.path()),
                &missing,
                "missing field",
            );
            missing[field] = Value::Null;
            refused(
                &mut command(root.path(), home.path()),
                &missing,
                if field == "keys" {
                    "inventory keys"
                } else {
                    "invalid type"
                },
            );
        }
        for field in ["configuration", "profile"] {
            let mut blank = original.clone();
            blank[field] = json!("  ");
            refused(
                &mut command(root.path(), home.path()),
                &blank,
                "marker-bound configuration",
            );
        }
        refused(
            &mut command(root.path(), home.path()),
            &original,
            "PLUMB_RELEASE_VERSION",
        );
    }
}

#[test]
fn identity() {
    let root = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("charts/probe")).unwrap();
    std::fs::write(
        root.path().join("charts/probe/Chart.yaml"),
        "name: probe\nversion: 1.2.0\nappVersion: '1.2.0'\n",
    )
    .unwrap();
    let binding = crate::marker::prepare(
        root.path(),
        home.path(),
        "[release]\nproduct='probe'\nauthority='https://releases.test'\n[release.chart]\nregistry='registry.example'\nchart='owner/probe'\naccount='Example'\n",
        "v1.2.0-beta.1",
    );
    let mut request = request();
    request["configuration"] = binding["configuration"].clone();
    request["profile"] = binding["profile"].clone();
    let request = crate::marker::planned(root.path(), home.path(), request, "v1.2.0-beta.1");
    let bound = || {
        let mut held = command(root.path(), home.path());
        held.env("PLUMB_RELEASE_VERSION", "v1.2.0-beta.1");
        held
    };
    for field in ["configuration", "profile"] {
        let mut drifted = request.clone();
        drifted[field] = json!("0".repeat(64));
        refused(&mut bound(), &drifted, "differs from its governance");
    }
    for field in ["action", "roots", "projections"] {
        let mut drifted = request.clone();
        drifted[field] = if field == "action" {
            json!("ship/elsewhere")
        } else {
            json!(["elsewhere"])
        };
        refused(
            &mut bound(),
            &drifted,
            &format!("{field} differs from its marker plan"),
        );
    }
    let mut drifted = request.clone();
    drifted["operation"] = json!({"type":"cargo"});
    refused(
        &mut bound(),
        &drifted,
        "not a declared marker-bound Ship request",
    );
    for field in ["workload", "proof", "publication"] {
        let mut drifted = request.clone();
        drifted["keys"][field] = json!("invalid");
        refused(&mut bound(), &drifted, "invalid hash");
    }
    let mut missing = request.clone();
    for field in ["workload", "proof", "publication"] {
        let mut drifted = request.clone();
        drifted["keys"][field] = json!("0".repeat(64));
        refused(&mut bound(), &drifted, "keys differ from its marker plan");
    }
    let mut drifted = request.clone();
    drifted["production"] = json!("0".repeat(64));
    refused(
        &mut bound(),
        &drifted,
        "production differs from its marker plan",
    );
    missing["keys"]
        .as_object_mut()
        .unwrap()
        .remove("publication");
    refused(&mut bound(), &missing, "no publication key");
    let mut malformed = request.clone();
    malformed["reuse"] = json!({"type":"none","source":"https://unexpected.test"});
    refused(&mut bound(), &malformed, "source does not match its type");
    refused(
        bound().env("PLUMB_RELEASE_CHANNEL", "stable"),
        &request,
        "channel differs",
    );
    refused(
        bound().env("PLUMB_RELEASE_COMMIT", "0".repeat(40)),
        &request,
        "commit differs",
    );
    refused(
        bound().env("PLUMB_RELEASE_ROOT", other.path()),
        &request,
        "root differs",
    );
    refused(
        bound().env("PLUMB_RELEASE_VERSION", "v1.2.0-beta.99"),
        &request,
        "v1.2.0-beta.99",
    );
    let chart = root.path().join("charts/probe/Chart.yaml");
    let original = std::fs::read(&chart).unwrap();
    std::fs::write(&chart, "name: probe\nversion: 1.2.1\n").unwrap();
    refused(&mut bound(), &request, "tracked tree differs");
    std::fs::write(&chart, original).unwrap();
    let output = Command::new("git")
        .arg("-C")
        .arg(root.path())
        .args(["commit", "--allow-empty", "-qm", "later checkout"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    refused(&mut bound(), &request, "checkout differs");
}
