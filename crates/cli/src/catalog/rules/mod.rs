use super::model::Rule;

pub mod consensus;
pub mod depot;
pub mod dispatch;
mod domain;
pub mod env;
pub mod prose;
pub mod release;
pub mod structure;
pub mod web;

pub use domain::{deps, vocabulary};

macro_rules! rule {
    (
        $name:ident, $id:literal, $summary:literal, $law:literal,
        $evidence:literal, Mechanized, $owner:ident, [$($tag:ident),+ $(,)?]
    ) => {
        pub static $name: $crate::catalog::model::Mechanism =
            $crate::catalog::model::Mechanism(
                $crate::catalog::model::Rule {
                    id: $id,
                    summary: $summary,
                    law: $law,
                    evidence: $evidence,
                    standing: $crate::catalog::model::Standing::Mechanized,
                    owner: &$crate::catalog::taxonomy::$owner,
                    tags: &[$(&$crate::catalog::taxonomy::$tag),+],
                },
            );
    };
    (
        $name:ident, $id:literal, $summary:literal, $law:literal,
        $evidence:literal, $standing:ident, $owner:ident, [$($tag:ident),+ $(,)?]
    ) => {
        pub static $name: $crate::catalog::model::Rule =
            $crate::catalog::model::Rule {
            id: $id,
            summary: $summary,
            law: $law,
            evidence: $evidence,
            standing: $crate::catalog::model::Standing::$standing,
            owner: &$crate::catalog::taxonomy::$owner,
            tags: &[$(&$crate::catalog::taxonomy::$tag),+],
        };
    };
}

pub(crate) use rule;

pub fn all() -> Vec<&'static Rule> {
    [
        consensus::all(),
        depot::all(),
        env::all(),
        structure::all(),
        deps::all(),
        web::all(),
        dispatch::all(),
        prose::all(),
        release::all(),
        vocabulary::all(),
    ]
    .into_iter()
    .flatten()
    .collect()
}
