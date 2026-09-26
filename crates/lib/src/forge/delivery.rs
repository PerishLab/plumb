use super::github::{self, Client, Issue};
use super::land::{self, Guard, Refusal};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const SCHEMA: &str = "plumb.delivery-plan/v1";

pub struct Request<'a> {
    pub root: &'a Path,
    pub base: &'a str,
    pub issue: &'a str,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Plan {
    pub schema: &'static str,
    pub root: PathBuf,
    pub repository: String,
    pub issue: Issue,
    #[serde(rename = "observed_at")]
    pub observed: u64,
    pub base: String,
    #[serde(rename = "base_commit")]
    pub target: String,
    pub branch: String,
    pub projection: String,
    pub source: String,
    pub candidate: String,
    pub pull: Pull,
    pub guard: Guard,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Pull {
    pub title: String,
    pub body: String,
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
    validate(&issue)?;
    let reference = format!("Refs {}", coordinate.render());
    let prepared = land::prepare(land::Request {
        root: request.root,
        base: request.base,
        title: &issue.title,
        body: &reference,
        watch: false,
    })?;
    let observed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| land::refuse("clock", format!("cannot observe time: {error}")))?
        .as_secs();
    Ok(Plan {
        schema: SCHEMA,
        root: prepared.root,
        repository,
        issue,
        observed,
        base: prepared.base,
        target: prepared.target,
        branch: prepared.branch,
        projection: prepared.projection,
        source: prepared.source,
        candidate: prepared.candidate,
        pull: Pull {
            title: prepared.title,
            body: prepared.body,
        },
        guard: prepared.guard,
    })
}

fn validate(issue: &Issue) -> Result<(), Refusal> {
    if issue.node.trim().is_empty() {
        return Err(land::refuse("issue", "issue has no stable node identity"));
    }
    if issue.kind.trim().is_empty() {
        return Err(land::refuse("issue", "issue has no enabled native type"));
    }
    if issue.state != "OPEN" {
        return Err(land::refuse(
            "issue",
            format!("issue #{} is not open", issue.number),
        ));
    }
    Ok(())
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
    use super::{Coordinate, validate};
    use crate::forge::github::Issue;

    #[test]
    fn coordinates() {
        let held = Coordinate::parse("PerishLab/plumb#20").expect("coordinate");
        assert_eq!(held.repository(), "PerishLab/plumb");
        assert_eq!(held.number, 20);
        for raw in ["plumb#20", "PerishLab/plumb", "PerishLab/plumb#0"] {
            assert!(Coordinate::parse(raw).is_err(), "{raw}");
        }
    }

    #[test]
    fn declarations() {
        let mut issue = Issue {
            node: "I_one".to_string(),
            number: 20,
            url: "https://github.com/PerishLab/plumb/issues/20".to_string(),
            title: "Plan delivery".to_string(),
            state: "OPEN".to_string(),
            kind: "Feature".to_string(),
            updated: "2026-09-26T00:00:00Z".to_string(),
            parent: None,
            parts: Vec::new(),
            behind: Vec::new(),
            blocking: Vec::new(),
        };
        assert!(validate(&issue).is_ok());
        issue.kind.clear();
        assert_eq!(validate(&issue).expect_err("type").kind, "issue");
        issue.kind = "Feature".to_string();
        issue.state = "CLOSED".to_string();
        assert_eq!(validate(&issue).expect_err("state").kind, "issue");
        issue.state = "OPEN".to_string();
        issue.node.clear();
        assert_eq!(validate(&issue).expect_err("node").kind, "issue");
    }
}
