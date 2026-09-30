mod inventory;
mod model;
mod process;
mod repo;
pub mod staging;

pub use model::{Checkout, Expectation, Inspection, Refusal, Relation, Worktree};

use std::path::Path;

pub fn inspect(root: &Path, expected: &Expectation) -> Result<Inspection, Refusal> {
    repo::Repository::open(root)?.inspect(expected)
}

pub fn advance(root: &Path, expected: &Expectation, observed: &str) -> Result<Inspection, Refusal> {
    repo::Repository::open(root)?.advance(expected, observed)
}
