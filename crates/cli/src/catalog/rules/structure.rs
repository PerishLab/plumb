use super::{Mechanism, rule};

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
rule!(DEFAULT_OVERRIDDEN, "structure.default-overridden");
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
#[rustfmt::skip]
pub fn mechanisms() -> Vec<&'static Mechanism> {
    vec![
        &ANCHOR_PRESENT, &ANCHOR_UNIQUE, &API_ENTRYPOINT, &BOUNDARY_EXISTS,
        &CARGO_TARGET_IGNORED, &CASCADE_DERIVES_IN_ANCHOR, &DEFAULT_OVERRIDDEN, &ECTROPY_POLICY,
        &ECTROPY_POLICY_PRESENT, &ECTROPY_POLICY_READABLE,
        &KNOWN_DIRECTORY, &KNOWN_FILE, &KNOWN_WORKFLOW,
        &SEAT_AFFIRMED, &SEAT_ANCHORED, &SEAT_MEMBER,
        &PACKAGE_DIRECTORY_NAME, &PACKAGE_UNDER_PACKAGES,
        &RESERVED_COMPONENTS_SEAT,
        &RETIRED_SEAT_ABSENT,
    ]
}
