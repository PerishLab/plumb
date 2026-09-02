use std::path::Path;

#[test]
fn versioned() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let support =
        std::fs::read_to_string(root.join("crates/cli/src/command/ship/transport/support.rs"))
            .expect("Plumb owns ship artifact contracts");
    for action in ["ship/cargo", "ship/oci"] {
        assert!(support.contains(action), "{action} must bind its version");
    }
    let execution =
        std::fs::read_to_string(root.join("crates/cli/src/command/ship/transport/execute.rs"))
            .expect("Plumb owns ship request execution");
    assert!(execution.contains("materialize(&artifacts, &workloads)"));
    assert!(!execution.contains("if release.channel == \"stable\""));
}
