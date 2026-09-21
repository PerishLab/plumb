use super::{Mechanism, rule};

rule!(DEPOT_SCHEMA, "depot.schema-supported");

pub fn mechanisms() -> Vec<&'static Mechanism> {
    vec![&DEPOT_SCHEMA]
}
