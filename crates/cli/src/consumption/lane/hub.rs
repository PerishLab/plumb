use super::plan::Plan;
use crate::command::{diagnostic, github};
use runseal::github::{Client, Dispatch};
use std::collections::BTreeMap;

const OWNER: &str = "PerishLab";
const REPO: &str = "wharf";

pub(super) trait Gateway {
    fn observe(&self, plan: &Plan) -> Result<(), String>;
    fn submit(&self, plan: &Plan) -> Result<(), String>;
}

pub(super) struct Native;

impl Gateway for Native {
    fn observe(&self, plan: &Plan) -> Result<(), String> {
        let client = github()?;
        let actor = client.viewer().map_err(diagnostic)?.value;
        let control = client
            .branch(OWNER, REPO, "main")
            .map_err(diagnostic)?
            .value;
        binding(plan, &actor.login, &control.commit.sha)?;
        let (commit, tree) = source(&client, plan)?;
        qualify(plan, &commit, &tree)
    }

    fn submit(&self, plan: &Plan) -> Result<(), String> {
        let input = Dispatch::new(
            "lane.yml",
            "main",
            BTreeMap::from([("request".into(), payload(plan)?)]),
        )
        .map_err(diagnostic)?;
        github()?
            .dispatch(OWNER, REPO, &input)
            .map_err(submission)?;
        Ok(())
    }
}

pub(super) fn binding(plan: &Plan, login: &str, head: &str) -> Result<(), String> {
    let workflow = format!("{OWNER}/{REPO}/.github/workflows/lane.yml@{head}");
    if login != plan.dispatch.caller() || workflow != plan.dispatch.workflow() {
        return Err("lane caller or fixed-main control moved; prepare and review again".into());
    }
    Ok(())
}

fn source(client: &Client, plan: &Plan) -> Result<(String, String), String> {
    let (owner, repo) = plan
        .source
        .repository()
        .split_once('/')
        .ok_or("invalid lane source repository")?;
    let commit = client
        .commit(owner, repo, plan.source.commit())
        .map_err(diagnostic)?
        .value;
    Ok((commit.sha, commit.tree.sha))
}

pub(super) fn qualify(plan: &Plan, commit: &str, tree: &str) -> Result<(), String> {
    if commit != plan.source.commit() || tree != plan.source.tree() {
        return Err("lane source is not the exact pushed commit/tree".into());
    }
    Ok(())
}

pub(super) fn payload(plan: &Plan) -> Result<String, String> {
    String::from_utf8(plan.dispatch.bytes()?).map_err(|_| "invalid lane dispatch encoding".into())
}

fn submission(error: runseal::github::Error) -> String {
    use runseal::github::Outcome;
    let followup = if error.outcome == Outcome::Unknown {
        "Wharf dispatch acknowledgment unverified; operation outcome unknown; do not retry, replay or repair"
    } else {
        "Wharf did not accept the dispatch"
    };
    format!("{followup}; {}", diagnostic(error))
}
