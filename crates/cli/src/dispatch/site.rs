mod artifact;
mod cloud;
mod inspect;
mod model;
mod plan;
mod process;
mod reach;
mod settings;
mod ship;

use std::path::Path;

pub(super) fn deploy(root: &Path) -> Result<String, String> {
    ship::run(root)
}

pub(super) fn inspect(root: &Path) -> Result<String, String> {
    inspect::run(root)
}

pub(super) fn plan(root: &Path) -> Result<String, String> {
    plan::run(root)
}
