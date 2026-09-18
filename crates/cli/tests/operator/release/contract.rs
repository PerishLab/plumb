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
        "PLUMB_BUILD_SOURCE=1 CARGO_PROFILE_DEV_DEBUG=0 CARGO_TARGET_DIR=\"$target\"",
        "--world 'debuginfo=0'",
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
        "$env:CARGO_PROFILE_DEV_DEBUG = '0'",
        "--world 'debuginfo=0'",
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
}
