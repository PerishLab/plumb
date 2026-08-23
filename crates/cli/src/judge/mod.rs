use self::finding::Finding;
use crate::shape;
use shape::Found;

pub(crate) mod depot;
mod deps;
mod dispatch;
mod env;
pub(crate) mod finding;
mod structure;
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
        lanes(&held.drift),
    ] {
        for seed in found {
            notes.push(Finding::new(seed));
        }
    }
    notes
}

fn lanes(drift: &[String]) -> Found {
    drift
        .iter()
        .cloned()
        .map(|evidence| {
            finding::Seed::noted(&crate::catalog::rules::release::LANE_RENDERED, evidence)
        })
        .collect()
}
