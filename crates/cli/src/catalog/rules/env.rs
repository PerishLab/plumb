use super::{Mechanism, rule};

rule!(EDITION_2024, "env.edition-2024");
rule!(ROLLING_CI_CONTAINER, "env.rolling-ci-container");

pub fn mechanisms() -> Vec<&'static Mechanism> {
    vec![&EDITION_2024, &ROLLING_CI_CONTAINER]
}
