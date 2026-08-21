use super::super::{Rule, rule};

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
    "First-party Deno dependencies carry no version requirement so their exact stable version is owned by the lock.",
    "Dependency requirements in Deno configuration.",
    Mechanized,
    PLUMB,
    [DEPENDENCY]
);
rule!(
    FIRST_PARTY_STABLE_LATEST,
    "deps.first-party-stable-latest",
    "First-party dependencies resolve to stable latest",
    "A direct dependency published by perish.code resolves exactly to the live latest non-prerelease, non-yanked version at its compiled registry authority, except the plumb substrate, whose currency the depot floor decides.",
    "Direct Deno and Cargo declarations, their exact lock resolutions, and live registry metadata.",
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
        &FIRST_PARTY_STABLE_LATEST,
        &STYLING_PACKAGE_ALLOWED,
    ]
}
