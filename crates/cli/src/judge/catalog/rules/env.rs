use super::{Rule, rule};

rule!(
    EDITION_2024,
    "env.edition-2024",
    "Rust uses edition 2024",
    "A Rust repository in the skeleton uses edition 2024.",
    "The workspace or package edition declared in Cargo.toml.",
    Mechanized,
    PLUMB,
    [CARGO_TAG, REPOSITORY]
);
rule!(
    ROLLING_CI_CONTAINER,
    "env.rolling-ci-container",
    "CI tracks the rolling workshop container",
    "The guard lane uses the workshop CI container without pinning a tag.",
    "The container image reference in the guard workflow.",
    Mechanized,
    PLUMB,
    [REPOSITORY]
);

pub fn all() -> Vec<&'static Rule> {
    vec![&EDITION_2024, &ROLLING_CI_CONTAINER]
}
