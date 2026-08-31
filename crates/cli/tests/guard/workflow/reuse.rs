use super::{PAIR, Plan, seat};

#[test]
fn reuse() {
    let root = seat("reuse-proof");
    root.declared(PAIR);
    root.git(&["commit", "-m", "source"]);
    let (cold, ok) = root.plan(None, &["runner=linux"]);
    assert!(ok, "{cold}");
    let cold: serde_json::Value = serde_json::from_str(&cold).expect("cold plan");
    let held = &cold["actions"][0];
    let inventory = serde_json::json!({
        "schema": "plumb.workflow-inventory/v1",
        "records": [{
            "action": held["name"],
            "workload": held["keys"]["workload"],
            "proof": held["keys"]["proof"],
            "source": {
                "type": "workload",
                "source": format!("r2://workloads/{}", held["keys"]["workload"].as_str().unwrap())
            }
        }, {
            "action": held["name"],
            "workload": held["keys"]["workload"],
            "proof": "0".repeat(64),
            "source": {
                "type": "workload",
                "source": format!("r2://workloads/{}", held["keys"]["workload"].as_str().unwrap())
            }
        }]
    });
    let path = root.inventory(&inventory.to_string());
    let (text, ok) = root.planned(Plan {
        base: None,
        world: &["runner=linux"],
        identity: &[],
        project: &[],
        roots: &[],
        inventory: Some(&path),
    });
    assert!(ok, "{text}");
    let plan: serde_json::Value = serde_json::from_str(&text).expect("plan");
    assert_eq!(plan["actions"][0]["run"], false);
    assert_eq!(plan["actions"][0]["decision"], "reuse");
    assert_eq!(plan["actions"][0]["reason"], "proof-held");
    assert_eq!(plan["actions"][0]["reuse"]["type"], "workload");

    let (moved, ok) = root.planned(Plan {
        base: None,
        world: &["runner=macos"],
        identity: &[],
        project: &[],
        roots: &[],
        inventory: Some(&path),
    });
    assert!(ok, "{moved}");
    let moved: serde_json::Value = serde_json::from_str(&moved).expect("moved plan");
    assert_eq!(moved["actions"][0]["run"], true);
    assert_eq!(moved["actions"][0]["reason"], "proof-moved");
    assert_eq!(moved["actions"][0]["reuse"]["type"], "workload");
}

#[test]
fn publication() {
    let root = seat("reuse-publication");
    root.declared(PAIR);
    root.git(&["commit", "-m", "source"]);
    let (cold, ok) = root.plan(None, &["runner=linux"]);
    assert!(ok, "{cold}");
    let cold: serde_json::Value = serde_json::from_str(&cold).expect("cold plan");
    let held = &cold["actions"][0];
    let workload = serde_json::json!({
        "action": held["name"],
        "workload": held["keys"]["workload"],
        "proof": held["keys"]["proof"],
        "source": {"type": "workload", "source": "r2://workloads/held"}
    });
    let path = root.inventory(
        &serde_json::json!({
            "schema": "plumb.workflow-inventory/v1",
            "records": [workload.clone()]
        })
        .to_string(),
    );
    let request = Plan {
        base: None,
        world: &["runner=linux"],
        identity: &["version=2.0.0"],
        project: &[],
        roots: &[],
        inventory: Some(&path),
    };
    let (publish, ok) = root.planned(request);
    assert!(ok, "{publish}");
    let publish: serde_json::Value = serde_json::from_str(&publish).expect("publish plan");
    let action = &publish["actions"][0];
    assert_eq!(action["run"], true);
    assert_eq!(action["reason"], "publication-moved");
    assert_eq!(action["reuse"]["type"], "workload");

    let url = serde_json::json!({
        "action": action["name"],
        "workload": action["keys"]["workload"],
        "proof": action["keys"]["proof"],
        "publication": action["keys"]["publication"],
        "source": {"type": "url", "source": "https://registry.example/artifact-2.0.0.tgz"},
        "depot": {
            "schema": "plumb.depot-worker/v1",
            "marker": "v2.0.0",
            "worker": "probe",
            "version": "worker-version"
        }
    });
    let path = root.inventory(
        &serde_json::json!({
            "schema": "plumb.workflow-inventory/v1",
            "records": [workload, url]
        })
        .to_string(),
    );
    let (held, ok) = root.planned(Plan {
        base: None,
        world: &["runner=linux"],
        identity: &["version=2.0.0"],
        project: &[],
        roots: &[],
        inventory: Some(&path),
    });
    assert!(ok, "{held}");
    let held: serde_json::Value = serde_json::from_str(&held).expect("held plan");
    assert_eq!(held["actions"][0]["run"], false);
    assert_eq!(held["actions"][0]["decision"], "skip");
    assert_eq!(held["actions"][0]["reason"], "publication-held");
    assert_eq!(held["actions"][0]["reuse"]["type"], "url");
    assert_eq!(
        held["actions"][0]["depot"]["schema"],
        "plumb.depot-worker/v1"
    );
}

#[test]
fn refusal() {
    let root = seat("inventory-refusal");
    root.declared(PAIR);
    root.git(&["commit", "-m", "source"]);
    let path = root.inventory(r#"{"schema":"plumb.workflow-inventory/v0","records":[]}"#);
    let (text, ok) = root.planned(Plan {
        base: None,
        world: &[],
        identity: &[],
        project: &[],
        roots: &[],
        inventory: Some(&path),
    });
    assert!(!ok, "{text}");

    let (cold, ok) = root.plan(None, &[]);
    assert!(ok, "{cold}");
    let cold: serde_json::Value = serde_json::from_str(&cold).expect("cold plan");
    let held = &cold["actions"][0];
    let path = root.inventory(
        &serde_json::json!({
            "schema": "plumb.workflow-inventory/v1",
            "records": [{
                "action": held["name"],
                "workload": held["keys"]["workload"],
                "source": {"type": "workload", "source": "r2://workloads/one"}
            }, {
                "action": held["name"],
                "workload": held["keys"]["workload"],
                "source": {"type": "workload", "source": "r2://workloads/two"}
            }]
        })
        .to_string(),
    );
    let (text, ok) = root.planned(Plan {
        base: None,
        world: &[],
        identity: &[],
        project: &[],
        roots: &[],
        inventory: Some(&path),
    });
    assert!(!ok, "{text}");
}
