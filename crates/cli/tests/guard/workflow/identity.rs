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
        "an embedded release version must rebuild across beta and stable"
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
