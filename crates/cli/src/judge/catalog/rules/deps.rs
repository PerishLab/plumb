use super::{Rule, rule};

rule!(
    RUST_BINARY_USES_CLAP,
    "deps.rust-binary-uses-clap",
    "Rust binaries use clap",
    "A shipped Rust binary exposes its command surface through clap.",
    "Cargo targets and the dependency graph.",
    Mechanized,
    PLUMB,
    [CARGO_TAG, DEPENDENCY]
);
rule!(
    RUST_BINARY_USES_PLUMB,
    "deps.rust-binary-uses-plumb",
    "Rust binaries use plumb",
    "A shipped Rust binary builds on the plumb substrate.",
    "Cargo targets and the dependency graph.",
    Mechanized,
    PLUMB,
    [CARGO_TAG, DEPENDENCY]
);
rule!(
    CURRENT_DEPENDENCY_NAME,
    "deps.current-dependency-name",
    "Dependencies use their current names",
    "A dependency renamed in the workshop is consumed under its current name.",
    "Dependency names in Deno configuration.",
    Mechanized,
    PLUMB,
    [DEPENDENCY]
);
rule!(
    SELF_BUILT_DEPENDENCY_UNPINNED,
    "deps.self-built-dependency-unpinned",
    "Self-built dependencies track latest",
    "Workspace-owned Deno dependencies without a declared support line are not pinned to a published version.",
    "Dependency requirements in Deno configuration.",
    Mechanized,
    PLUMB,
    [DEPENDENCY]
);
rule!(
    SUPPORTED_DEPENDENCY_LINE,
    "deps.supported-dependency-line",
    "Supported dependencies use an admitted line",
    "A dependency with a compiled support declaration uses an admitted requirement and resolves to an admitted locked version.",
    "The dependency requirement in Deno configuration and its matching frozen lock resolution.",
    Mechanized,
    PLUMB,
    [DEPENDENCY]
);
rule!(
    STYLING_PACKAGE_ALLOWED,
    "deps.styling-package-allowed",
    "Web styling stays inside the design system",
    "A web package does not bypass the design system with a blacklisted styling package.",
    "Node dependency maps across workspace packages.",
    Mechanized,
    WEB,
    [DEPENDENCY, WEB_TAG]
);

pub fn all() -> Vec<&'static Rule> {
    vec![
        &CURRENT_DEPENDENCY_NAME,
        &RUST_BINARY_USES_CLAP,
        &RUST_BINARY_USES_PLUMB,
        &SELF_BUILT_DEPENDENCY_UNPINNED,
        &SUPPORTED_DEPENDENCY_LINE,
        &STYLING_PACKAGE_ALLOWED,
    ]
}
