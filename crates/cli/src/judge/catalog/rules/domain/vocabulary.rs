use super::super::{Rule, rule};

rule!(
    RETIRED_TERM_ABSENT,
    "vocabulary.retired-term-absent",
    "Retired domain terms leave the active closure",
    "A term in Plumb's transitional retired dictionary is absent from Git-tracked paths and current working-tree bytes outside docs/CHANGELOG.",
    "The bundled p64-v1 dictionary compared with tracked path and file bytes.",
    Mechanized,
    PLUMB,
    [REPOSITORY, VOCABULARY_TAG]
);

pub fn all() -> Vec<&'static Rule> {
    vec![&RETIRED_TERM_ABSENT.0]
}
