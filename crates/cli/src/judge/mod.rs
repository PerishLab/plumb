use self::finding::Finding;
use crate::shape;

pub(crate) mod depot;
mod deps;
mod dispatch;
mod env;
pub(crate) mod finding;
pub(crate) mod structure;
mod text;
pub(crate) mod vocabulary;
mod web;

pub use text::show;

pub fn judge(held: &shape::Shape) -> Vec<Finding> {
    let mut notes = Vec::new();
    for found in [
        env::judge(held),
        structure::judge(held),
        deps::check(held),
        web::judge(held.web.as_ref()),
        dispatch::judge(held.dispatch.as_ref()),
        structure::lane::judge(&held.lane),
    ] {
        for seed in found {
            notes.push(Finding::new(seed));
        }
    }
    notes
}
