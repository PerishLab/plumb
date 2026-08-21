use super::super::{Mechanism, rule};

rule!(RUST_BINARY_USES_CLAP, "deps.rust-binary-uses-clap");
rule!(RUST_BINARY_USES_PLUMB, "deps.rust-binary-uses-plumb");
rule!(CURRENT_DEPENDENCY_NAME, "deps.current-dependency-name");
rule!(
    SELF_BUILT_DEPENDENCY_UNPINNED,
    "deps.self-built-dependency-unpinned"
);
rule!(FIRST_PARTY_STABLE_LATEST, "deps.first-party-stable-latest");
rule!(STYLING_PACKAGE_ALLOWED, "deps.styling-package-allowed");

pub fn mechanisms() -> Vec<&'static Mechanism> {
    vec![
        &CURRENT_DEPENDENCY_NAME,
        &RUST_BINARY_USES_CLAP,
        &RUST_BINARY_USES_PLUMB,
        &SELF_BUILT_DEPENDENCY_UNPINNED,
        &FIRST_PARTY_STABLE_LATEST,
        &STYLING_PACKAGE_ALLOWED,
    ]
}
