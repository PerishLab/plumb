use super::{Mechanism, rule};

rule!(EDITION_2024, "env.edition-2024");

pub fn mechanisms() -> Vec<&'static Mechanism> {
    vec![&EDITION_2024]
}
