mod assess;
pub(in crate::command) mod channel;
mod collect;
mod facts;
mod identity;

pub use identity::deed::Deed;
use std::path::Path;

use crate::shape::release::Spec;

pub fn run(deed: Deed) -> i32 {
    match &deed {
        Deed::Policy => return facts::Facts::policy(),
        Deed::Review {
            product,
            reference,
            purpose,
        } => return facts::Facts::review(product, reference, *purpose),
        Deed::Inspect {
            input,
            purpose,
            product,
        } => {
            return facts::Facts::inspect(input, *purpose, product.as_deref());
        }
        _ => {}
    }
    if let Deed::Collect { repository, commit } = &deed {
        return collect::Collect { repository, commit }.run();
    }
    if let Deed::Assess { input } = &deed {
        return assess::Assess(input).run();
    }
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
        Deed::Policy | Deed::Review { .. } | Deed::Inspect { .. } => {
            Err("facts use their read-only entry point".into())
        }
        Deed::Collect { .. } => Err("collection uses its read-only entry point".into()),
        Deed::Assess { .. } => Err("assessment uses its read-only entry point".into()),
        Deed::Stamp {
            version,
            remote,
            dry,
        } => super::operator::stamp(&version, &remote, dry),
        Deed::Retract { version, dry } => super::operator::retract(&version, dry),
        Deed::Rejoin => super::operator::rejoin(),
        Deed::Open {
            version,
            remote,
            dry,
        } => super::operator::open(&version, &remote, dry),
        Deed::Close {
            version,
            abandon,
            remote,
            dry,
        } => super::operator::close(&version, abandon, &remote, dry),
        Deed::Owed { version, remote } => super::operator::owed(version.as_deref(), &remote),
        Deed::Authority { json } => guard(json),
    }
}

pub(crate) fn channel(version: &str) -> Result<String, String> {
    channel::channel(version)
}

pub(super) fn authority(root: &Path) -> Result<String, String> {
    Spec::controller(root).map(|spec| spec.authority)
}

fn guard(json: bool) -> Result<String, String> {
    let authority = plumb::guard::Authority::running()?;
    if json {
        return serde_json::to_string(&authority)
            .map_err(|error| format!("cannot encode Guard authority: {error}"));
    }
    Ok(format!(
        "producer {}\ndepot {}",
        authority.producer(),
        authority.depot()
    ))
}
