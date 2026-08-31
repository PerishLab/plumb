mod artifact;
mod cloud;
mod model;
mod process;
mod reach;
mod settings;
mod worker;

use std::path::Path;

pub(in crate::command) fn deploy(
    root: &Path,
    worker: &str,
    version: &str,
) -> Result<String, String> {
    worker::deploy(root, worker, version)
}

pub(super) use worker::Seat as Worker;
