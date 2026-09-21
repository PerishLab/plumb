pub(in crate::command) mod channel;
pub(in crate::command) mod cloudflare;
mod identity;
pub(in crate::command) mod output;
mod truth;

pub use identity::deed::Deed;
use std::path::Path;
pub(super) use truth::{manager, record};

use crate::shape::release::Spec;

pub(in crate::command) struct Product<'a>(&'a Spec);

impl<'a> Product<'a> {
    pub fn new(spec: &'a Spec) -> Self {
        Self(spec)
    }

    pub fn depot(&self) -> truth::depot::Source<'_> {
        truth::depot::Source {
            product: &self.0.product,
            authority: &self.0.authority,
        }
    }
}

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
        Deed::Managers { version, out } => {
            let spec = Spec::read(Path::new("plumb.toml"))?;
            manager::write(&spec, &channel::channel(&version)?, &version, &out)
        }
    }
}

pub(crate) fn channel(version: &str) -> Result<String, String> {
    channel::channel(version)
}

pub(in crate::command) use truth::depot::validate as validate_depot;

pub(super) fn authority(root: &Path) -> Result<String, String> {
    Spec::controller(root).map(|spec| spec.authority)
}
