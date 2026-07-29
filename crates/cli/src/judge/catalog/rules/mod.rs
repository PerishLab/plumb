use super::model::Rule;

pub mod consensus;
pub mod deps;
pub mod dispatch;
pub mod env;
pub mod lock;
pub mod prose;
pub mod structure;
pub mod web;

macro_rules! rule {
    (
        $name:ident, $id:literal, $summary:literal, $law:literal,
        $evidence:literal, Mechanized, $owner:ident, [$($tag:ident),+ $(,)?]
    ) => {
        pub static $name: $crate::judge::catalog::model::Mechanism =
            $crate::judge::catalog::model::Mechanism(
                $crate::judge::catalog::model::Rule {
                    id: $id,
                    summary: $summary,
                    law: $law,
                    evidence: $evidence,
                    standing: $crate::judge::catalog::model::Standing::Mechanized,
                    owner: &$crate::judge::catalog::taxonomy::$owner,
                    tags: &[$(&$crate::judge::catalog::taxonomy::$tag),+],
                },
            );
    };
    (
        $name:ident, $id:literal, $summary:literal, $law:literal,
        $evidence:literal, $standing:ident, $owner:ident, [$($tag:ident),+ $(,)?]
    ) => {
        pub static $name: $crate::judge::catalog::model::Rule =
            $crate::judge::catalog::model::Rule {
            id: $id,
            summary: $summary,
            law: $law,
            evidence: $evidence,
            standing: $crate::judge::catalog::model::Standing::$standing,
            owner: &$crate::judge::catalog::taxonomy::$owner,
            tags: &[$(&$crate::judge::catalog::taxonomy::$tag),+],
        };
    };
}

pub(crate) use rule;

pub fn all() -> Vec<&'static Rule> {
    [
        consensus::all(),
        env::all(),
        structure::all(),
        deps::all(),
        web::all(),
        dispatch::all(),
        lock::all(),
        prose::all(),
    ]
    .into_iter()
    .flatten()
    .collect()
}
