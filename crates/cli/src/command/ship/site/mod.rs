mod artifact;
mod cloud;
mod model;
mod process;
mod reach;
mod settings;
mod worker;

use crate::shape::release::Spec;
use std::path::Path;

pub(in crate::command) fn deploy(
    root: &Path,
    spec: &Spec,
    worker: &str,
    version: &str,
) -> Result<String, String> {
    worker::deploy(root, spec, worker, version)
}

pub(super) use worker::Seat as Worker;
