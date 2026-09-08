use std::path::Path;

fn text(path: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read_to_string(root.join(path)).unwrap_or_else(|error| panic!("read {path}: {error}"))
}

#[test]
fn recovery() {
    let held = text(".forgejo/scripts/bootstrap-plumb.sh");
    let windows = text(".forgejo/scripts/bootstrap-plumb.ps1");
    let installed = held
        .find("cp \"$target/debug/plumb\" \"$bin/plumb\"")
        .unwrap();
    for configured in held
        .match_indices("  install_configuration")
        .map(|(at, _)| at)
    {
        assert!(
            installed < configured,
            "only the exact atom may install configuration"
        );
    }
    assert_eq!(held.matches("  install_configuration").count(), 2);
    assert!(held.contains("if \"$tool\" configuration --help >/dev/null 2>&1"));
    assert!(windows.contains("& $tool configuration --help *> $null"));

    let handoff = held
        .find("if [ -z \"$source\" ] && [ -n \"$handoff\" ]")
        .unwrap();
    let manager = held
        .find("manager=\"$RUNNER_TEMP/manage-plumb.sh\"")
        .unwrap();
    assert!(
        handoff < manager,
        "an exact handoff must bypass stable bootstrap"
    );
    for recovery in [
        "stable Plumb is unavailable; cold-building the exact atom",
        "workflow plan --help",
        "confirmed exact Plumb atom visibility",
    ] {
        assert!(held.contains(recovery), "Unix bootstrap omits {recovery}");
        assert!(
            windows.contains(recovery),
            "Windows bootstrap omits {recovery}"
        );
    }
    assert!(held.contains("[ -x \"$tool\" ]"));
    assert!(windows.contains("if (Test-Path $tool)"));
    assert!(windows.contains("catch {\n      $managerInstalled = $false"));

    for binding in [
        "PLUMB_BUILD_VERSION is required",
        "PLUMB_BUILD_COMMIT is required",
        "plumb-atom-$PLUMB_BUILD_COMMIT",
        "PLUMB_BUILD_SOURCE=1 CARGO_TARGET_DIR=\"$target\"",
        "--root 'ship/atom=*'",
        "--workload \"commit=$PLUMB_BUILD_COMMIT\"",
        "workflow record ship/atom",
        "PLUMB_ATOM_SOURCE",
        "PLUMB_ATOM_HANDOFF",
        "jq -r '.type'",
        "inventory_base=${inventory_base%/inventory.json}",
        "--retry 30",
        "v1/channels/stable.json",
        "PLUMB_HOME=\"$RUNNER_TEMP/plumb-home-$configuration\"",
        "install_configuration \"$PLUMB_HOME/configurations\"",
    ] {
        assert!(
            held.contains(binding),
            "Unix atom bootstrap omits {binding}"
        );
    }
    for binding in [
        "PLUMB_BUILD_VERSION",
        "PLUMB_BUILD_COMMIT",
        "plumb-atom-$env:PLUMB_BUILD_COMMIT",
        "$env:CARGO_TARGET_DIR = $target",
        "$env:PLUMB_BUILD_SOURCE = '1'",
        "--root 'ship/atom=*'",
        "--workload \"commit=$env:PLUMB_BUILD_COMMIT\"",
        "PLUMB_ATOM_SOURCE",
        "PLUMB_ATOM_HANDOFF",
        "ConvertFrom-Json",
        "-replace '/inventory\\.json$', ''",
        "$env:PLUMB_HOME = Join-Path $env:RUNNER_TEMP",
        "Join-Path $env:PLUMB_HOME 'configurations'",
    ] {
        assert!(
            windows.contains(binding),
            "Windows atom bootstrap omits {binding}"
        );
    }
    assert!(!held.contains("PLUMB_RELEASE_"));
    assert!(!windows.contains("PLUMB_RELEASE_"));
}

#[test]
fn versioned() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let support =
        std::fs::read_to_string(root.join("crates/cli/src/command/ship/transport/support.rs"))
            .expect("Plumb owns ship artifact contracts");
    assert!(support.contains("\"ship/cargo\" => Contract::Version"));
    assert!(support.contains("\"ship/oci\" => Contract::Portable"));
    assert!(
        support.contains("action == \"ship/cargo\""),
        "only package workloads embed their projected version"
    );
    let execution =
        std::fs::read_to_string(root.join("crates/cli/src/command/ship/transport/execute.rs"))
            .expect("Plumb owns ship request execution");
    for image in ["false", "true"] {
        assert!(execution.contains(&format!(
            "materialize(&artifacts, &workloads, governance.marker(), {image})"
        )));
    }
    assert!(!execution.contains("if release.channel == \"stable\""));

    let trigger = text("crates/cli/src/command/operator/trigger.rs");
    assert!(
        !trigger.contains("before.product == \"plumb\""),
        "Plumb must not bind controller configuration to the product marker: {trigger}"
    );
    assert!(
        trigger.contains("let configuration = plumb::depot::rules()?"),
        "every ship must use the dispatching Plumb atom's configuration: {trigger}"
    );
    assert!(
        trigger.contains("\"plumb\": plumb::version!(\"PLUMB\")"),
        "{trigger}"
    );
    let workflow = text(".forgejo/workflows/ship.yml");
    assert!(workflow.contains("configuration:"), "{workflow}");
    assert!(workflow.contains("inputs.configuration"), "{workflow}");
    assert!(workflow.contains("secrets.SHIP_PUBLISH_S3_ACCESS_KEY"));
    assert!(workflow.contains("secrets.SHIP_PUBLISH_FINGERPRINT"));
    assert!(!workflow.contains("PLUMB_PUBLISH_BUCKET:"));
    assert!(support.contains("!held.bucket.is_empty() && held.bucket != bucket"));
    assert!(support.contains("derived.bucket = bucket"));
    let resolver = text("crates/cli/src/command/ship/transport/resolve.rs");
    assert!(!resolver.contains("workload: Some(&marker.version)"));
    assert!(!resolver.contains("workload: Some(&marker.commit)"));
    assert!(resolver.contains("workload: Some(base)"));
    assert!(resolver.contains("release: Some(base)"));
    assert_eq!(
        resolver
            .matches("workload: Some(self.marker.base())")
            .count(),
        1
    );
    assert!(resolver.contains("then_some(self.marker.base())"));
    let execution = text("crates/cli/src/command/ship/transport/production.rs");
    assert!(
        execution.contains("channel::base(&marker.version)"),
        "binary execution must derive its reusable version instead of trusting the plan"
    );
    assert!(execution.contains("channel: \"stable\""));

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
