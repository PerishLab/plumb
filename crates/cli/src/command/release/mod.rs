pub(in crate::command) mod channel;
mod identity;
mod truth;

pub use identity::deed::Deed;
use std::path::Path;
pub(super) use truth::manager;

use crate::shape::release::Spec;

pub fn run(deed: Deed) -> i32 {
    let result = execute(deed);
    match result {
        Ok(message) => {
            println!("{message}");
            0
        }
        Err(error) => {
            eprintln!("plumb release: {error}");
            1
        }
    }
}

fn execute(deed: Deed) -> Result<String, String> {
    match deed {
        Deed::Stamp {
            version,
            remote,
            dry,
        } => super::operator::stamp(&version, &remote, dry),
        Deed::Retract { version, dry } => super::operator::retract(&version, dry),
        Deed::Rejoin { dry } => super::operator::rejoin(dry),
        Deed::Open {
            version,
            from,
            remote,
            dry,
        } => super::operator::open(&version, &from, &remote, dry),
        Deed::Close {
            version,
            abandon,
            remote,
            dry,
        } => super::operator::close(&version, abandon, &remote, dry),
        Deed::Owed { version, remote } => super::operator::owed(version.as_deref(), &remote),
        Deed::Managers { version, out } => {
            let spec = Spec::read(Path::new("plumb.toml"))?;
            manager::write(&spec, &channel::channel(&version)?, &version, &out)
        }
    }
}

pub(crate) fn channel(version: &str) -> Result<String, String> {
    channel::channel(version)
}

pub(super) fn authority(root: &Path) -> Result<String, String> {
    Spec::controller(root).map(|spec| spec.authority)
}
