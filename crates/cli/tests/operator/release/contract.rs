use std::path::Path;

fn text(path: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read_to_string(root.join(path)).unwrap_or_else(|error| panic!("read {path}: {error}"))
}

#[test]
fn versioned() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let support =
        std::fs::read_to_string(root.join("crates/cli/src/command/ship/transport/support.rs"))
            .expect("Plumb owns ship artifact contracts");
    for action in ["ship/cargo", "ship/oci"] {
        assert!(support.contains(action), "{action} must bind its version");
    }
    assert!(
        support.contains("action == \"ship/cargo\""),
        "only package workloads embed their projected version"
    );
    let execution =
        std::fs::read_to_string(root.join("crates/cli/src/command/ship/transport/execute.rs"))
            .expect("Plumb owns ship request execution");
    assert!(execution.contains("materialize(&artifacts, &workloads)"));
    assert!(!execution.contains("if release.channel == \"stable\""));

    let trigger = text("crates/cli/src/command/operator/trigger.rs");
    assert!(trigger.contains("before.product == \"plumb\""), "{trigger}");
    assert!(trigger.contains("before.marker.clone()"), "{trigger}");
    assert!(
        trigger.contains("plumb::version!(\"PLUMB\").to_string()"),
        "{trigger}"
    );
    let workflow = text(".forgejo/workflows/ship.yml");
    assert!(workflow.contains("configuration:"), "{workflow}");
    assert!(workflow.contains("inputs.configuration"), "{workflow}");
    let resolver = text("crates/cli/src/command/ship/transport/resolve.rs");
    assert!(resolver.contains("workload: Some(&marker.version)"));
    assert!(!resolver.contains("workload: Some(marker.base())"));
    assert_eq!(
        resolver
            .matches("workload: Some(self.marker.base())")
            .count(),
        1
    );
    assert!(resolver.contains("then_some(self.marker.base())"));

    let bootstrap = text("crates/cli/src/command/guard/precommit/configuration.rs");
    assert!(
        bootstrap.contains("PLUMB_GUARD_RECOVERY_VALIDATOR"),
        "Plumb must retain a checked recovery path when its released validator is broken"
    );
    let recovery = text("crates/cli/src/command/release/truth/compatibility.rs");
    assert!(recovery.contains("!executable.is_absolute()"));
    assert!(recovery.contains("binding.release.version"));
    assert!(recovery.contains("record::digest(executable)"));
    let validation = text("crates/cli/src/command/release/truth/depot.rs");
    assert!(validation.contains(".env_remove(\"PLUMB_HOME\")"));
    assert!(validation.contains(".env(\"PLUMB_GUARD_DEPOT\", &seat)"));

    let configuration = text("crates/cli/src/command/depot/configuration.rs");
    assert!(configuration.contains("release.validator(&marker.version, true)"));
    assert!(!configuration.contains("release.validator(&marker.marker, true)"));
    assert!(configuration.contains("validate_depot(spec, &binding, &plan, recovery)"));
}

#[test]
fn split() {
    let release = text("crates/cli/help/release.txt");
    assert!(release.contains("Ship never invokes\ndepot"), "{release}");
    assert!(
        release.contains("Use depot separately with\nthe same marker"),
        "{release}"
    );

    let scenario = text("skills/plumb/SCENARIOS.md");
    let configuration = scenario
        .find("plumb depot configuration --marker VERSION")
        .expect("configuration is explicit");
    let ship = scenario
        .find("plumb ship dispatch --marker VERSION")
        .expect("ship is explicit");
    let channel = scenario
        .find("plumb depot channel --marker VERSION")
        .expect("stable activation is explicit");
    assert!(configuration < ship && ship < channel, "{scenario}");
    assert!(scenario.contains("Neither command invokes the other."));

    let structure = text("crates/cli/rules/structure.toml");
    assert!(
        structure.contains("remaining an independent local transaction"),
        "{structure}"
    );

    let catalog = text("crates/cli/rules/catalog.toml");
    assert!(
        catalog.contains("Each ship dispatch names and resolves one exact Plumb atom"),
        "{catalog}"
    );
    assert!(
        catalog.contains("changing that atom neither mutates the product marker"),
        "{catalog}"
    );
    assert!(!catalog.contains("A release marker records the exact Plumb atom"));
}
