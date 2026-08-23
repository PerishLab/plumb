use self::finding::Finding;
use crate::shape;
use shape::Found;

pub(crate) mod depot;
mod deps;
mod env;
pub(crate) mod finding;
mod structure;
mod text;
pub(crate) mod vocabulary;

pub use text::show;

pub fn judge(held: &shape::Shape) -> Vec<Finding> {
    let mut notes = Vec::new();
    for found in [
        env::judge(held),
        structure::judge(held),
        deps::check(held),
        held.web.clone().unwrap_or_default(),
        held.dispatch.clone().unwrap_or_default(),
        lanes(held),
    ] {
        for seed in found {
            notes.push(Finding::new(seed));
        }
    }
    notes
}

fn lanes(held: &shape::Shape) -> Found {
    shape::lane::Seat(&held.root)
        .drift()
        .into_iter()
        .map(|evidence| {
            finding::Seed::noted(&crate::catalog::rules::release::LANE_RENDERED, evidence)
        })
        .collect()
}
