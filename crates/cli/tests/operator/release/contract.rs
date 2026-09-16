use std::path::Path;

fn text(path: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read_to_string(root.join(path)).unwrap_or_else(|error| panic!("read {path}: {error}"))
}

#[test]
fn policy() {
    let authority = text("crates/cli/src/command/release/authority/model.rs");
    assert!(authority.contains("crate::shape::product::names()?"));
    let catalog = text("crates/cli/src/shape/repository/product/catalog.rs");
    assert!(catalog.contains("seat.read(\"rules/products.toml\", FACTORY)"));
    assert!(!authority.contains("crates/cli/rules/products.toml"));
    let home = super::support::depot(&[]);
    let rules = plumb::depot::Rules::at(
        &home.path().join("configurations"),
        plumb::version!("PLUMB"),
    )
    .unwrap();
    for path in [
        "rules/products.toml",
        "rules/seat.toml",
        "rules/workflow.toml",
    ] {
        assert_eq!(rules.read(path).unwrap(), super::support::policy(path));
    }
}

#[test]
fn recovery() {
    let held = text(".forgejo/scripts/lib/runner.py");
    assert!(held.contains("receipt.exists()"));
    assert!(held.contains("result = decode(receipt.read_bytes())"));
    assert!(held.contains("self.retain(receipt, result)"));
    let completion = text(".forgejo/scripts/lib/inventory.py");
    assert!(completion.contains("action key has conflicting completion evidence"));
    assert!(completion.contains("held = self.store.read(route)"));
    let controller = text(".forgejo/scripts/controller.py");
    assert!(controller.contains("CARGO_PROFILE_DEV_DEBUG=\"0\""));
    assert!(controller.contains("PLUMB_BUILD_SOURCE=\"1\""));
    assert!(!controller.contains("workflow plan"));
}

#[test]
fn versioned() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let support =
        std::fs::read_to_string(root.join("crates/cli/src/command/ship/transport/support.rs"))
            .expect("Plumb owns ship artifact contracts");
    let execution =
        std::fs::read_to_string(root.join("crates/cli/src/command/ship/request/execute.rs"))
            .expect("Plumb owns ship request execution");
    for image in ["false", "true"] {
        assert!(execution.contains(&format!(
            "materialize(&artifacts, workloads, marker, {image})"
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
    let declaration = text("crates/cli/src/command/ship/request/declaration.rs");
    assert!(declaration.contains("self.implementation(\"produce\")?"));
    assert!(declaration.contains("\"content\": {\"node\": producer"));
    let execution = text("crates/cli/src/command/ship/transport/production.rs");
    assert!(!execution.contains("channel: \"stable\""));

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
    let public = text("crates/cli/src/command/release/truth/verify.rs");
    assert!(public.contains("\"--max-time\""));
    assert!(public.contains("\"--retry-max-time\""));

    let configuration = text("crates/cli/src/command/depot/configuration.rs");
    assert!(configuration.contains("release.validator(&marker.version, true)"));
    assert!(!configuration.contains("release.validator(&marker.marker, true)"));
    assert!(configuration.contains("crate::command::release::validate_depot("));
    assert!(configuration.contains("request.recovery.as_deref()"));
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

    let structure = super::support::policy("rules/structure.toml");
    assert!(
        structure.contains("remaining an independent local transaction"),
        "{structure}"
    );

    let catalog = super::support::policy("rules/catalog.toml");
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
