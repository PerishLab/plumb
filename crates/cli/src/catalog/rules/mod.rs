use super::model::{Rule, Standing};
use super::taxonomy::{Owner, Tag};
use crate::command::depot::held;
use std::sync::LazyLock;

pub mod depot;
pub mod dispatch;
mod domain;
pub mod env;
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
        stated(),
        depot::all(),
        env::all(),
        structure::all(),
        deps::all(),
        web::all(),
        dispatch::all(),
        release::all(),
        vocabulary::all(),
    ]
    .into_iter()
    .flatten()
    .collect()
}

static STATED: LazyLock<Vec<Rule>> = LazyLock::new(|| {
    let factory = plumb::seat::resource!("rules/catalog.toml");
    let text = held()
        .read(SEAT, factory)
        .unwrap_or_else(|_| factory.to_string());
    let doc: toml::Table = text
        .parse()
        .unwrap_or_else(|error| panic!("{SEAT} does not parse: {error}"));
    doc.get("rule")
        .and_then(toml::Value::as_array)
        .unwrap_or_else(|| panic!("{SEAT} must hold a rule array"))
        .iter()
        .map(carry)
        .collect()
});

const SEAT: &str = "rules/catalog.toml";

pub fn stated() -> Vec<&'static Rule> {
    STATED.iter().collect()
}

fn carry(value: &toml::Value) -> Rule {
    let id = word(value, "id");
    Rule {
        id,
        summary: word(value, "summary"),
        law: word(value, "law"),
        evidence: word(value, "evidence"),
        standing: Standing::parse(word(value, "standing"))
            .unwrap_or_else(|| panic!("{SEAT}: rule {id} names an unknown standing")),
        owner: owner(word(value, "owner"), id),
        tags: tags(value, id),
    }
}

fn word(value: &toml::Value, key: &str) -> &'static str {
    let held = value
        .get(key)
        .and_then(toml::Value::as_str)
        .unwrap_or_else(|| panic!("{SEAT}: a rule has no {key}"));
    Box::leak(held.to_string().into_boxed_str())
}

fn owner(id: &str, rule: &str) -> &'static Owner {
    super::taxonomy::OWNERS
        .iter()
        .copied()
        .find(|owner| owner.id == id)
        .unwrap_or_else(|| panic!("{SEAT}: rule {rule} names unknown owner {id}"))
}

fn tags(value: &toml::Value, rule: &'static str) -> &'static [&'static Tag] {
    let held = value
        .get("tags")
        .and_then(toml::Value::as_array)
        .unwrap_or_else(|| panic!("{SEAT}: rule {rule} has no tags"));
    let held = held
        .iter()
        .map(|value| {
            let id = value
                .as_str()
                .unwrap_or_else(|| panic!("{SEAT}: rule {rule} names a tag that is not a string"));
            super::taxonomy::TAGS
                .iter()
                .copied()
                .find(|tag| tag.id == id)
                .unwrap_or_else(|| panic!("{SEAT}: rule {rule} names unknown tag {id}"))
        })
        .collect::<Vec<_>>();
    Box::leak(held.into_boxed_slice())
}
