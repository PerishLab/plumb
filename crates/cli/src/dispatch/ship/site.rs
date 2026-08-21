mod artifact;
mod cloud;
mod deploy;
mod inspect;
mod model;
mod plan;
mod process;
mod settings;
mod worker;

use std::path::Path;

pub(super) fn deploy(root: &Path) -> Result<String, String> {
    deploy::run(root)
}

pub(super) fn inspect(root: &Path) -> Result<String, String> {
    inspect::run(root)
}

pub(super) fn plan(root: &Path) -> Result<String, String> {
    plan::run(root)
}

pub(super) fn worker(seat: worker::Seat<'_>, publish: bool) -> Result<String, String> {
    if publish {
        seat.publish()
    } else {
        seat.rehearse()
    }
}

pub(super) use worker::Seat as Worker;
