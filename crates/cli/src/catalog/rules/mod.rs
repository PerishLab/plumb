use super::model::{Mechanism, Rule};

pub mod depot;
pub mod dispatch;
mod domain;
pub mod env;
pub mod release;
pub mod structure;
pub mod web;

pub use domain::{deps, vocabulary};

macro_rules! rule {
    ($name:ident, $id:literal) => {
        pub static $name: $crate::catalog::model::Mechanism =
            $crate::catalog::model::Mechanism($id);
    };
}

pub(crate) use rule;

pub fn all() -> Vec<&'static Rule> {
    super::held().rules.clone()
}

pub fn held(id: &str) -> &'static Rule {
    super::seek(&super::held().rules, id)
        .unwrap_or_else(|| panic!("the catalogue holds no rule {id}"))
}

pub(super) fn mechanisms() -> Vec<&'static Mechanism> {
    [
        depot::mechanisms(),
        dispatch::mechanisms(),
        env::mechanisms(),
        structure::mechanisms(),
        deps::mechanisms(),
        web::mechanisms(),
        release::mechanisms(),
        vocabulary::mechanisms(),
    ]
    .into_iter()
    .flatten()
    .collect()
}
