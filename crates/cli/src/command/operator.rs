mod course;
mod mark;
mod rejoin;
mod value;
mod wharf;
mod worktree;

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

pub(super) fn rejoin(dry: bool) -> Result<String, String> {
    rejoin::rejoin(dry)
}
