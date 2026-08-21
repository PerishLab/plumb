use super::{Mechanism, rule};

rule!(DEPOT_PUBLISHED, "depot.roots-published");
rule!(DEPOT_SCHEMA, "depot.schema-supported");

pub fn mechanisms() -> Vec<&'static Mechanism> {
    vec![&DEPOT_PUBLISHED, &DEPOT_SCHEMA]
}
