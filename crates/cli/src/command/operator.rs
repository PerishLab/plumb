mod consign;
mod course;
mod line;
mod mark;
mod owed;
mod rejoin;
mod value;
mod wharf;
mod worktree;

use clap::Args;

pub use consign::Consign;
pub(super) use line::{close, open, owed};
pub(super) use mark::retract;
pub(super) use rejoin::rejoin;
pub(super) use wharf::stamp;
pub(super) use wharf::status::status;

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

pub fn consign(options: Consign) -> Result<String, String> {
    consign::consign(options)
}
