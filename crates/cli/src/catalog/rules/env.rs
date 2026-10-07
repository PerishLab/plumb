use super::{Mechanism, rule};

rule!(EDITION_2024, "env.edition-2024");
rule!(IMMUTABLE_CI_CONTAINER, "env.immutable-ci-container");
rule!(TOOLCHAIN_DOMAIN, "env.toolchain-domain");

pub fn mechanisms() -> Vec<&'static Mechanism> {
    vec![&EDITION_2024, &IMMUTABLE_CI_CONTAINER, &TOOLCHAIN_DOMAIN]
}
