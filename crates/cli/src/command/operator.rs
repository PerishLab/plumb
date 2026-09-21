mod course;
mod mark;
mod value;
mod wharf;

use clap::Args;

#[derive(Args)]
#[group(skip)]
pub struct Dispatch {
    #[arg(long)]
    marker: String,
    #[arg(long, default_value = "")]
    repo: String,
    #[arg(long)]
    watch: bool,
    #[arg(long = "dry-run")]
    dry: bool,
}

pub fn dispatch(options: Dispatch) -> Result<String, String> {
    wharf::dispatch(options)
}

pub(super) fn stamp(version: &str, remote: &str, dry: bool) -> Result<String, String> {
    wharf::stamp(version, remote, dry)
}

pub(super) fn retract(version: &str, dry: bool) -> Result<String, String> {
    mark::retract(version, dry)
}
