#[test]
fn bootstrap() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let held = std::fs::read_to_string(
        root.join("crates/cli/src/command/guard/precommit/configuration.rs"),
    )
    .expect("configuration bootstrap");
    assert!(held.contains("Context::Source => source.latest(\"stable\", true)?"));
    assert!(held.contains("Ok(branch) if branch.starts_with(\"release/\")"));
    let action =
        std::fs::read_to_string(root.join("crates/cli/src/command/guard/precommit/action.rs"))
            .expect("precommit action");
    let mismatch = action.find("let mismatched").expect("mismatch decision");
    let reuse = action
        .find("if !mismatched")
        .expect("conditioned proof reuse");
    assert!(mismatch < reuse);
    assert!(action.find(".checks()?").expect("current actions") < reuse);
}
