use super::{Plan, now, validate};
use crate::forge::github::{self, Client, Issue};
use crate::forge::land::{self, Guard, Refusal};
use serde::Serialize;
use std::path::{Path, PathBuf};

pub const SCHEMA: &str = "plumb.delivery-land/v1";

pub struct Landing<'a> {
    pub root: &'a Path,
    pub plan: &'a Plan,
    pub watch: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Report {
    pub schema: &'static str,
    pub root: PathBuf,
    pub repository: String,
    pub issue: Issue,
    #[serde(rename = "observed_at")]
    pub observed: u64,
    #[serde(rename = "landed_at")]
    pub landed: u64,
    pub base: String,
    pub candidate: String,
    pub guard: Guard,
    pub pull: Pull,
    pub merged: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Pull {
    pub node: String,
    pub number: u64,
    pub url: String,
}

pub fn land(request: Landing<'_>) -> Result<Report, Refusal> {
    let plan = request.plan;
    if plan.schema != super::SCHEMA {
        return Err(land::refuse(
            "plan",
            format!(
                "delivery plan schema {} is not {}",
                plan.schema,
                super::SCHEMA
            ),
        ));
    }
    let remote = github::remote(request.root).map_err(|error| land::refuse("remote", error))?;
    let repository = format!("{}/{}", remote.owner, remote.repo);
    if repository != plan.repository {
        return Err(land::refuse(
            "stale",
            format!(
                "delivery plan repository {} is not {repository}",
                plan.repository
            ),
        ));
    }
    let issue = Client::new(&remote)
        .issue(plan.issue.number)
        .map_err(|error| land::refuse("forge", error))?;
    validate(&issue)?;
    if issue != plan.issue {
        return Err(land::refuse(
            "stale",
            "delivery plan Issue identity or declarations changed",
        ));
    }
    let expected = land::Preparation {
        root: plan.root.clone(),
        base: plan.base.clone(),
        target: plan.target.clone(),
        branch: plan.branch.clone(),
        projection: plan.projection.clone(),
        source: plan.source.clone(),
        candidate: plan.candidate.clone(),
        title: plan.pull.title.clone(),
        body: plan.pull.body.clone(),
        guard: plan.guard.clone(),
    };
    let report = land::Request {
        root: request.root,
        base: &plan.base,
        title: &plan.pull.title,
        body: &plan.pull.body,
        watch: request.watch,
    }
    .exact(&expected)?;
    Ok(Report {
        schema: SCHEMA,
        root: report.root,
        repository,
        issue,
        observed: plan.observed,
        landed: now()?,
        base: report.base,
        candidate: report.candidate,
        guard: plan.guard.clone(),
        pull: Pull {
            node: report.node,
            number: report.pull,
            url: report.url,
        },
        merged: report.merged,
    })
}

pub fn read(path: &Path) -> Result<Plan, Refusal> {
    let bytes = std::fs::read(path).map_err(|error| {
        land::refuse(
            "plan",
            format!("cannot read delivery plan {}: {error}", path.display()),
        )
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|error| land::refuse("plan", format!("cannot parse delivery plan: {error}")))
}

pub fn required(root: &Path) -> Result<bool, Refusal> {
    let path = root.join("plumb.toml");
    if !path.is_file() {
        return Ok(false);
    }
    let text = std::fs::read_to_string(&path).map_err(|error| {
        land::refuse("policy", format!("cannot read {}: {error}", path.display()))
    })?;
    let value: toml::Value = toml::from_str(&text).map_err(|error| {
        land::refuse(
            "policy",
            format!("cannot parse {}: {error}", path.display()),
        )
    })?;
    let Some(delivery) = value.get("delivery") else {
        return Ok(false);
    };
    let Some(plan) = delivery.get("plan") else {
        return Ok(false);
    };
    plan.as_bool().ok_or_else(|| {
        land::refuse(
            "policy",
            format!("{}.delivery.plan must be true or false", path.display()),
        )
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn policies() {
        let fixture = tempfile::tempdir().expect("fixture");
        assert!(!super::required(fixture.path()).expect("absent"));
        std::fs::write(
            fixture.path().join("plumb.toml"),
            "[delivery]\nplan = true\n",
        )
        .expect("policy");
        assert!(super::required(fixture.path()).expect("required"));
    }
}
