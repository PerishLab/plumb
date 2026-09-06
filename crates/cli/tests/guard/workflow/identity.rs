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
        &["source=0123456789abcdef"],
    );
    assert!(ok, "{beta}");
    let (stable, ok) = root.workload(
        request(&["runner=docker", "release=v1.2.0"], &["marker=v1.2.0"]),
        &["source=0123456789abcdef"],
    );
    assert!(ok, "{stable}");
    let (moved, ok) = root.workload(
        request(&["runner=docker", "release=v1.2.0"], &["marker=v1.2.0"]),
        &["source=fedcba9876543210"],
    );
    assert!(ok, "{moved}");
    let beta: serde_json::Value = serde_json::from_str(&beta).expect("beta plan");
    let stable: serde_json::Value = serde_json::from_str(&stable).expect("stable plan");
    let moved: serde_json::Value = serde_json::from_str(&moved).expect("moved plan");
    assert_eq!(
        beta["actions"][0]["keys"]["workload"], stable["actions"][0]["keys"]["workload"],
        "one source commit must cross beta and stable marker identities"
    );
    assert_ne!(
        beta["actions"][0]["keys"]["proof"], stable["actions"][0]["keys"]["proof"],
        "marker worlds must retain distinct proofs"
    );
    assert_ne!(
        beta["actions"][0]["keys"]["publication"], stable["actions"][0]["keys"]["publication"],
        "marker identities must retain distinct publications"
    );
    assert_ne!(
        stable["actions"][0]["keys"]["workload"], moved["actions"][0]["keys"]["workload"],
        "a changed source identity must rebuild"
    );
}
