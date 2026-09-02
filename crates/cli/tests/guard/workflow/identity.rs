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
    let beta: serde_json::Value = serde_json::from_str(&beta).expect("beta plan");
    let stable: serde_json::Value = serde_json::from_str(&stable).expect("stable plan");
    assert_ne!(
        beta["actions"][0]["keys"]["workload"], stable["actions"][0]["keys"]["workload"],
        "version-bound workloads must not cross marker identities"
    );
}
