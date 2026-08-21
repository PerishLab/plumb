use super::super::{Mechanism, rule};

rule!(RETIRED_TERM_ABSENT, "vocabulary.retired-term-absent");

pub fn mechanisms() -> Vec<&'static Mechanism> {
    vec![&RETIRED_TERM_ABSENT]
}
