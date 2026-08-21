use super::{Rule, rule};

rule!(
    DEPOT_PUBLISHED,
    "depot.roots-published",
    "The depot carries what this repository records",
    "Every configuration root this repository records is present in the held depot version under the same digest.",
    "Git-recorded root objects compared with the manifest of the held depot version.",
    Mechanized,
    PLUMB,
    [CONFIGURATION, REPOSITORY]
);
rule!(
    DEPOT_SCHEMA,
    "depot.schema-supported",
    "The running binary meets the depot floor",
    "The running plumb is at or above the version the held depot manifest declares as its schema floor.",
    "The running version compared with the schema floor of the held depot manifest.",
    Mechanized,
    PLUMB,
    [CONFIGURATION, ADOPTION]
);

pub fn all() -> Vec<&'static Rule> {
    vec![&DEPOT_PUBLISHED.0, &DEPOT_SCHEMA.0]
}
