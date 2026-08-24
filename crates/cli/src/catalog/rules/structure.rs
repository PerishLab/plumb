use super::{Mechanism, rule};

mod guard;
mod site;

pub use guard::{
    GUARD_CHECKS_RELEASE_PROFILE, GUARD_CONCURRENCY, GUARD_RUNS_DOCTOR, GUARD_RUNS_ECTROPY,
    GUARD_USES_CURRENT_ECTROPY_MODE,
};
pub use site::SITE_DEPLOY_LANE;

rule!(GUARD_LANE_PRESENT, "structure.guard-lane-present");
rule!(ECTROPY_POLICY_PRESENT, "structure.ectropy-policy-present");
rule!(PACKAGE_UNDER_PACKAGES, "structure.package-under-packages");
rule!(PACKAGE_DIRECTORY_NAME, "structure.package-directory-name");
rule!(
    RESERVED_COMPONENTS_SEAT,
    "structure.reserved-components-seat"
);
rule!(ECTROPY_POLICY_READABLE, "structure.ectropy-policy-readable");
rule!(ECTROPY_POLICY, "structure.ectropy-policy");
rule!(KNOWN_FILE, "structure.known-file");
rule!(SEAT_ANCHORED, "structure.seat-anchored");
rule!(SEAT_MEMBER, "structure.seat-member");
rule!(SEAT_AFFIRMED, "structure.seat-affirmed");
rule!(KNOWN_DIRECTORY, "structure.known-directory");
rule!(KNOWN_WORKFLOW, "structure.known-workflow");
rule!(RETIRED_SEAT_ABSENT, "structure.retired-seat-absent");
rule!(BOUNDARY_EXISTS, "structure.boundary-exists");
rule!(ANCHOR_PRESENT, "structure.anchor-present");
rule!(ANCHOR_UNIQUE, "structure.anchor-unique");
rule!(API_ENTRYPOINT, "structure.api-entrypoint");
rule!(
    CASCADE_DERIVES_IN_ANCHOR,
    "structure.cascade-derives-in-anchor"
);
rule!(CARGO_TARGET_IGNORED, "structure.cargo-target-ignored");
rule!(RELEASE_LANE_PRESENT, "structure.release-lane-present");
rule!(RELEASE_SOURCE_BOUND, "structure.release-source-bound");
#[rustfmt::skip]
pub fn mechanisms() -> Vec<&'static Mechanism> {
    vec![
        &ANCHOR_PRESENT, &ANCHOR_UNIQUE, &API_ENTRYPOINT, &BOUNDARY_EXISTS,
        &CARGO_TARGET_IGNORED, &CASCADE_DERIVES_IN_ANCHOR, &ECTROPY_POLICY,
        &ECTROPY_POLICY_PRESENT, &ECTROPY_POLICY_READABLE,
        &GUARD_CHECKS_RELEASE_PROFILE, &GUARD_CONCURRENCY,
        &GUARD_RUNS_DOCTOR, &GUARD_RUNS_ECTROPY, &GUARD_USES_CURRENT_ECTROPY_MODE,
        &GUARD_LANE_PRESENT, &KNOWN_DIRECTORY, &KNOWN_FILE, &KNOWN_WORKFLOW,
        &SEAT_AFFIRMED, &SEAT_ANCHORED, &SEAT_MEMBER,
        &PACKAGE_DIRECTORY_NAME, &PACKAGE_UNDER_PACKAGES,
        &RELEASE_LANE_PRESENT, &RELEASE_SOURCE_BOUND, &RESERVED_COMPONENTS_SEAT,
        &RETIRED_SEAT_ABSENT,
        &SITE_DEPLOY_LANE,
    ]
}
