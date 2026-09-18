use super::{PAIR, Plan, seat};

#[test]
fn binding() {
    let root = seat("workload-binding");
    root.declared(PAIR);
    root.git(&["commit", "-m", "source"]);
    let request = |world, identity| Plan {
        base: None,
        world,
        identity,
        project: &[],
        roots: &[],
        inventory: None,
    };
    let (beta, ok) = root.workload(
        request(
            &["runner=docker", "release=v1.2.0-beta.1"],
            &["marker=v1.2.0-beta.1"],
        ),
        &["release=v1.2.0-beta.1"],
    );
    assert!(ok, "{beta}");
    let (stable, ok) = root.workload(
        request(&["runner=docker", "release=v1.2.0"], &["marker=v1.2.0"]),
        &["release=v1.2.0"],
    );
    assert!(ok, "{stable}");
    let (retry, ok) = root.workload(
        request(&["runner=docker", "release=v1.2.0"], &["marker=v1.2.0"]),
        &["release=v1.2.0"],
    );
    assert!(ok, "{retry}");
    let beta: serde_json::Value = serde_json::from_str(&beta).expect("beta plan");
    let stable: serde_json::Value = serde_json::from_str(&stable).expect("stable plan");
    let retry: serde_json::Value = serde_json::from_str(&retry).expect("retry plan");
    assert_ne!(
        beta["actions"][0]["keys"]["workload"], stable["actions"][0]["keys"]["workload"],
        "identity-bound output must differ across beta and stable"
    );
    assert_ne!(
        beta["actions"][0]["keys"]["proof"], stable["actions"][0]["keys"]["proof"],
        "marker worlds must retain distinct proofs"
    );
    assert_ne!(
        beta["actions"][0]["keys"]["publication"], stable["actions"][0]["keys"]["publication"],
        "marker identities must retain distinct publications"
    );
    assert_eq!(
        stable["actions"][0]["keys"]["workload"], retry["actions"][0]["keys"]["workload"],
        "one exact effective release version must remain reusable"
    );
}

#[test]
fn content() {
    let root = seat("identity-content");
    root.declared(PAIR);
    root.git(&["commit", "-m", "source"]);
    let plan = |fields: &[&str], marker: &str| {
        let (text, ok) = root.workload(
            Plan {
                base: None,
                world: fields,
                identity: &[marker],
                project: &[],
                roots: &[],
                inventory: None,
            },
            fields,
        );
        assert!(ok, "{text}");
        serde_json::from_str::<serde_json::Value>(&text).unwrap()["actions"][0]["keys"].clone()
    };
    let fields = [
        "phase=content/v1",
        "release=v1.2.0",
        "target=linux",
        "runner=docker",
    ];
    let beta = plan(&fields, "marker=v1.2.0-beta.1");
    let stable = plan(&fields, "marker=v1.2.0");
    assert_eq!(beta["workload"], stable["workload"]);
    assert_eq!(beta["proof"], stable["proof"]);
    assert_ne!(beta["publication"], stable["publication"]);
    let bound = plan(
        &[
            "phase=identity/v1",
            "release=v1.2.0",
            "target=linux",
            "runner=docker",
        ],
        "marker=v1.2.0",
    );
    assert_ne!(stable["workload"], bound["workload"]);
    let platform = plan(
        &[
            "phase=content/v1",
            "release=v1.2.0",
            "target=windows",
            "runner=windows",
        ],
        "marker=v1.2.0",
    );
    assert_ne!(stable["workload"], platform["workload"]);
}
