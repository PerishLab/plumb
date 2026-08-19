mod course;
mod datum;
mod line;
mod mark;
mod pick;
mod rejoined;
mod trigger;
mod value;

use clap::Args;

#[derive(Args)]
#[group(skip)]
pub struct Dispatch {
    #[arg(long)]
    version: String,
    #[arg(long, default_value = "")]
    repo: String,
    #[arg(long)]
    watch: bool,
    #[arg(long = "dry-run")]
    dry: bool,
}

pub fn dispatch(options: Dispatch) -> Result<String, String> {
    trigger::run(options)
}

pub(super) fn line(deed: super::release::Deed) -> Result<String, String> {
    line::run(deed)
}

pub(super) fn stamp(version: &str, dry: bool) -> Result<String, String> {
    mark::stamp(version, dry)
}

pub(super) fn retract(version: &str, dry: bool) -> Result<String, String> {
    mark::retract(version, dry)
}
