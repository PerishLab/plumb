use super::github::{self, Client};
use super::land::{self, Refusal};
use crate::delivery::{self as kernel, Narrative};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub mod landing;

pub use crate::delivery::{Plan, SCHEMA};
pub use landing::{Landing, Report, land, read, required};

pub struct Request<'a> {
    pub root: &'a Path,
    pub base: &'a str,
    pub issue: &'a str,
    pub close: bool,
}

pub fn plan(request: Request<'_>) -> Result<Plan, Refusal> {
    let remote = github::remote(request.root).map_err(|error| land::refuse("remote", error))?;
    let coordinate = Coordinate::parse(request.issue)?;
    let repository = format!("{}/{}", remote.owner, remote.repo);
    if coordinate.repository() != repository {
        return Err(land::refuse(
            "issue",
            format!(
                "issue {} belongs to {}, not delivery repository {repository}",
                request.issue,
                coordinate.repository()
            ),
        ));
    }
    let issue = Client::new(&remote)
        .issue(coordinate.number)
        .map_err(|error| land::refuse("forge", error))?;
    let pull = Narrative {
        title: issue.title.clone(),
        body: association(request.close, &coordinate),
    };
    let authority =
        crate::guard::Authority::running().map_err(|error| land::refuse("guard", error))?;
    kernel::prepare(
        kernel::Request {
            root: request.root,
            repository: &repository,
            issue: &issue,
            observed: now()?,
            base: request.base,
            pull: &pull,
        },
        &authority,
    )
}

fn association(close: bool, coordinate: &Coordinate) -> String {
    format!(
        "{} {}",
        if close { "Closes" } else { "Refs" },
        coordinate.render()
    )
}

pub(super) fn now() -> Result<u64, Refusal> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| land::refuse("clock", format!("cannot observe time: {error}")))
        .map(|held| held.as_secs())
}

struct Coordinate {
    owner: String,
    repository: String,
    number: u64,
}

impl Coordinate {
    fn parse(raw: &str) -> Result<Self, Refusal> {
        let (seat, number) = raw
            .split_once('#')
            .ok_or_else(|| land::refuse("issue", "issue must be OWNER/REPOSITORY#NUMBER"))?;
        let (owner, repository) = seat
            .split_once('/')
            .filter(|(owner, repository)| {
                !owner.is_empty() && !repository.is_empty() && !repository.contains('/')
            })
            .ok_or_else(|| land::refuse("issue", "issue must be OWNER/REPOSITORY#NUMBER"))?;
        let number = number
            .parse::<u64>()
            .ok()
            .filter(|number| *number > 0)
            .ok_or_else(|| land::refuse("issue", "issue number must be a positive integer"))?;
        Ok(Self {
            owner: owner.to_string(),
            repository: repository.to_string(),
            number,
        })
    }

    fn repository(&self) -> String {
        format!("{}/{}", self.owner, self.repository)
    }

    fn render(&self) -> String {
        format!("{}#{}", self.repository(), self.number)
    }
}

#[cfg(test)]
mod tests {
    use super::{Coordinate, association};

    #[test]
    fn coordinates() {
        let held = Coordinate::parse("PerishLab/plumb#20").expect("coordinate");
        assert_eq!(held.repository(), "PerishLab/plumb");
        assert_eq!(association(false, &held), "Refs PerishLab/plumb#20");
        assert_eq!(association(true, &held), "Closes PerishLab/plumb#20");
        for raw in ["plumb#20", "PerishLab/plumb", "PerishLab/plumb#0"] {
            assert!(Coordinate::parse(raw).is_err(), "{raw}");
        }
    }
}
