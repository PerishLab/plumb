use super::{Mechanism, rule};

rule!(PLUMB_CURRENT, "config.plumb-current-and-guarded");

pub fn mechanisms() -> Vec<&'static Mechanism> {
    vec![&PLUMB_CURRENT]
}
