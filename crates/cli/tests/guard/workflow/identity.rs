use super::{PAIR, Plan, seat};

#[test]
fn binding() {
    let root = seat("workload-binding");
    root.declared(PAIR);
    root.git(&["commit", "-m", "source"]);
    let request = || Plan {
        base: None,
        world: &["runner=docker"],
        identity: &["marker=v1.2.0"],
        project: &[],
        roots: &[],
        inventory: None,
    };
    let (beta, ok) = root.workload(request(), &["release=v1.2.0-beta.1"]);
    assert!(ok, "{beta}");
    let (stable, ok) = root.workload(request(), &["release=v1.2.0"]);
    assert!(ok, "{stable}");
    let (portable, ok) = root.workload(request(), &[]);
    assert!(ok, "{portable}");
    let beta: serde_json::Value = serde_json::from_str(&beta).expect("beta plan");
    let stable: serde_json::Value = serde_json::from_str(&stable).expect("stable plan");
    let portable: serde_json::Value = serde_json::from_str(&portable).expect("portable plan");
    assert_ne!(
        beta["actions"][0]["keys"]["workload"], stable["actions"][0]["keys"]["workload"],
        "version-bound workloads must not cross marker identities"
    );
    assert_eq!(
        portable["actions"][0]["keys"]["workload"], portable["actions"][0]["after"],
        "an unbound workload must retain its source key"
    );
}
