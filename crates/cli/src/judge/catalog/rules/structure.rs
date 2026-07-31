use super::{Rule, rule};

mod guard;
mod site;

pub use guard::{
    GUARD_CHECKS_RELEASE_PROFILE, GUARD_CONCURRENCY, GUARD_RUNS_DOCTOR, GUARD_RUNS_ECTROPY,
    GUARD_USES_CURRENT_ECTROPY_MODE,
};
pub use site::{SITE_DEPLOY_LANE, SITE_SHIP_WRAPPER};

rule!(
    GUARD_LANE_PRESENT,
    "structure.guard-lane-present",
    "Guard has one workflow lane",
    "A governed repository carries the canonical guard workflow without requiring a repository-local wrapper.",
    "The .forgejo/workflows/guard.yml seat.",
    Mechanized,
    PLUMB,
    [ADOPTION, REPOSITORY]
);
rule!(
    ECTROPY_POLICY_PRESENT,
    "structure.ectropy-policy-present",
    "Ectropy policy is present",
    "A governed repository declares its ectropy policy.",
    "The repository-root ectropy.toml seat.",
    Mechanized,
    ECTROPY,
    [ECTROPY_TAG, REPOSITORY]
);
rule!(
    INIT_REQUIRES_PLUMB,
    "structure.init-requires-plumb",
    "Init requires plumb",
    "The init wrapper installs or resolves plumb before using it.",
    "The init wrapper source.",
    Mechanized,
    PLUMB,
    [ADOPTION, REPOSITORY]
);
rule!(
    PACKAGE_UNDER_PACKAGES,
    "structure.package-under-packages",
    "Publishable packages sit under packages",
    "A publishable Node package lives below packages/ rather than at repository root.",
    "Package manifests at the root and below packages/.",
    Mechanized,
    PLUMB,
    [REPOSITORY]
);
rule!(
    PACKAGE_DIRECTORY_NAME,
    "structure.package-directory-name",
    "Package directory matches package name",
    "The final package name segment matches its directory under packages/.",
    "Package manifest names and their directory seats.",
    Mechanized,
    PLUMB,
    [REPOSITORY]
);
rule!(
    RESERVED_COMPONENTS_SEAT,
    "structure.reserved-components-seat",
    "Reusable components stay in the design system",
    "packages/components is reserved; reusable components belong to the design system.",
    "The packages/components directory seat.",
    Mechanized,
    WEB,
    [REPOSITORY, WEB_TAG]
);
rule!(
    ECTROPY_POLICY_READABLE,
    "structure.ectropy-policy-readable",
    "Ectropy policy is readable",
    "The shape checker must be able to parse the repository's ectropy policy.",
    "Parsing ectropy.toml.",
    Mechanized,
    ECTROPY,
    [ECTROPY_TAG, REPOSITORY]
);
rule!(
    ECTROPY_POLICY,
    "structure.ectropy-policy",
    "Ectropy policy matches repository shape",
    "The repository policy is reconciled with the grants and limits its current shape earns.",
    "The declared policy compared with plumb's rendered policy.",
    Mechanized,
    ECTROPY,
    [ECTROPY_TAG, REPOSITORY]
);
rule!(
    KNOWN_DIRECTORY,
    "structure.known-directory",
    "Top-level directories have a known role",
    "Every top-level directory has a shadow in the skeleton or a matching workflow.",
    "Repository top-level directory names.",
    Mechanized,
    PLUMB,
    [REPOSITORY]
);
rule!(
    OPERATOR_TEST_OWNED_BY_SEALKIT,
    "structure.operator-test-owned-by-sealkit",
    "Operator tests live with sealkit",
    "Tests of .runseal behavior belong to sealkit; product repositories keep only thin wrappers.",
    "Test files found below .runseal.",
    Mechanized,
    PLUMB,
    [ADOPTION, REPOSITORY]
);
rule!(
    KNOWN_WRAPPER,
    "structure.known-wrapper",
    "Operator wrappers have known roles",
    "Every .runseal wrapper has a shadow in the skeleton.",
    "Wrapper names under .runseal/wrappers.",
    Mechanized,
    PLUMB,
    [ADOPTION, REPOSITORY]
);
rule!(
    KNOWN_WORKFLOW,
    "structure.known-workflow",
    "Workflows have known roles",
    "Every workflow has a shadow in the skeleton.",
    "Workflow names under the repository's CI directory.",
    Mechanized,
    PLUMB,
    [REPOSITORY]
);
rule!(
    INIT_PATHS_READABLE,
    "structure.init-paths-readable",
    "Init requirements are readable",
    "When an init wrapper exists, plumb can read the wrapper paths it requires.",
    "The init wrapper's required path declaration.",
    Mechanized,
    PLUMB,
    [ADOPTION, REPOSITORY]
);
rule!(
    INIT_REQUIRES_EXISTING_WRAPPER,
    "structure.init-requires-existing-wrapper",
    "Init names existing wrappers",
    "Every wrapper required by init exists in the repository.",
    "Init requirements compared with .runseal/wrappers.",
    Mechanized,
    PLUMB,
    [ADOPTION, REPOSITORY]
);
rule!(
    BOUNDARY_EXISTS,
    "structure.boundary-exists",
    "Declared boundaries exist",
    "Every path named as a repository boundary exists.",
    "Boundary paths declared in repository policy.",
    Mechanized,
    ECTROPY,
    [ECTROPY_TAG, REPOSITORY]
);
rule!(
    ANCHOR_PRESENT,
    "structure.anchor-present",
    "Rust repositories carry an anchor crate",
    "A Rust repository has one crate named after the repository.",
    "Cargo package names across the workspace.",
    Mechanized,
    PLUMB,
    [CARGO_TAG, REPOSITORY]
);
rule!(
    ANCHOR_UNIQUE,
    "structure.anchor-unique",
    "The anchor crate is unique",
    "Exactly one workspace crate is named after the repository.",
    "Cargo package names across the workspace.",
    Mechanized,
    PLUMB,
    [CARGO_TAG, REPOSITORY]
);
rule!(
    API_ENTRYPOINT,
    "structure.api-entrypoint",
    "API crates are executable",
    "A crate named api carries an executable entrypoint.",
    "Cargo targets and source seats in api crates.",
    Mechanized,
    PLUMB,
    [CARGO_TAG, DISPATCH]
);
rule!(
    CASCADE_DERIVES_IN_ANCHOR,
    "structure.cascade-derives-in-anchor",
    "Cascade derives live in the anchor",
    "Repository runtime vocabulary is declared in the anchor crate, not a leaf.",
    "Cascade derive sites compared with anchor and executable api crates.",
    Mechanized,
    PLUMB,
    [CARGO_TAG, CONFIGURATION]
);
rule!(
    CARGO_TARGET_IGNORED,
    "structure.cargo-target-ignored",
    "Cargo target output is ignored",
    "A Rust repository ignores target/ at its root.",
    "Cargo.toml presence and exact target/ lines in .gitignore.",
    Mechanized,
    CARGO,
    [CARGO_TAG, REPOSITORY]
);
rule!(
    RELEASE_LANE_PRESENT,
    "structure.release-lane-present",
    "Release surfaces have canonical lanes",
    "A binary release declaration is paired with exact-candidate and stable-consensus workflows.",
    "The release table in plumb.toml and workflow names.",
    Mechanized,
    RELEASE,
    [RELEASE_TAG, REPOSITORY]
);
rule!(
    RELEASE_SOURCE_BOUND,
    "structure.release-source-bound",
    "Release callers leave one event source",
    "Exact and stable product workflows expose and forward no source or second ref input; the called shared workflow binds its direct event ref and commit.",
    "The release-exact.yml and release-stable.yml caller inputs and workflow_call bindings, plus the called workflow event context.",
    Mechanized,
    RELEASE,
    [RELEASE_TAG, REPOSITORY]
);
rule!(
    REGISTRY_RELEASE_WRAPPER_PRESENT,
    "structure.registry-release-wrapper-present",
    "Registry releases have an operator wrapper",
    "A repository publishing registry packages exposes one operator wrapper.",
    "Published package declarations and the wrapper seat.",
    Mechanized,
    RELEASE,
    [ADOPTION, RELEASE_TAG]
);
#[rustfmt::skip]
pub fn all() -> Vec<&'static Rule> {
    vec![
        &ANCHOR_PRESENT, &ANCHOR_UNIQUE, &API_ENTRYPOINT, &BOUNDARY_EXISTS,
        &CARGO_TARGET_IGNORED, &CASCADE_DERIVES_IN_ANCHOR, &ECTROPY_POLICY,
        &ECTROPY_POLICY_PRESENT, &ECTROPY_POLICY_READABLE,
        &GUARD_CHECKS_RELEASE_PROFILE, &GUARD_CONCURRENCY,
        &GUARD_RUNS_DOCTOR, &GUARD_RUNS_ECTROPY, &GUARD_USES_CURRENT_ECTROPY_MODE,
        &INIT_PATHS_READABLE, &INIT_REQUIRES_EXISTING_WRAPPER, &INIT_REQUIRES_PLUMB,
        &GUARD_LANE_PRESENT, &KNOWN_DIRECTORY, &KNOWN_WORKFLOW, &KNOWN_WRAPPER,
        &OPERATOR_TEST_OWNED_BY_SEALKIT, &PACKAGE_DIRECTORY_NAME, &PACKAGE_UNDER_PACKAGES,
        &REGISTRY_RELEASE_WRAPPER_PRESENT, &RELEASE_LANE_PRESENT, &RELEASE_SOURCE_BOUND,
        &RESERVED_COMPONENTS_SEAT, &SITE_DEPLOY_LANE, &SITE_SHIP_WRAPPER,
    ]
}
