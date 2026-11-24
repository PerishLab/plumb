use crate::config::github::diagnostic;
use crate::delivery::{Guard, Patience, Squash};
use runseal::github::{Client, Filter, Invocation, Outcome, Target};
use serde_json::Value;
use std::collections::BTreeSet;
use std::time::Instant;

pub fn guard(
    client: &Client,
    owner: &str,
    repo: &str,
    guard: &Guard<'_>,
) -> Result<String, String> {
    let rows = checks(client, owner, repo, guard.head)?;
    guard.observe(&rows, |id| {
        let id = id
            .parse()
            .map_err(|_| "Guard check has no workflow run identity")?;
        let invocation = Invocation::new(id, None).map_err(diagnostic)?;
        let run = client
            .workflow(owner, repo, invocation)
            .map_err(diagnostic)?
            .value;
        serde_json::to_value(run).map_err(|error| error.to_string())
    })
}

fn checks(client: &Client, owner: &str, repo: &str, head: &str) -> Result<Vec<Value>, String> {
    let mut checks = Vec::new();
    let mut ids = BTreeSet::new();
    let mut total = None;
    for page in 1..=100 {
        let filter = Filter::new(head, page, 100).map_err(diagnostic)?;
        let result = client
            .checks(owner, repo, &filter)
            .map_err(diagnostic)?
            .value;
        if total
            .replace(result.total)
            .is_some_and(|count| count != result.total)
        {
            return Err("check-run inventory changed during paging".into());
        }
        let empty = result.rows.is_empty();
        for check in result.rows {
            if !ids.insert(check.id) {
                return Err("check-run inventory repeated an identity".into());
            }
            checks.push(serde_json::to_value(check).map_err(|error| error.to_string())?);
        }
        match checks.len().cmp(&(result.total as usize)) {
            std::cmp::Ordering::Equal => return Ok(checks),
            std::cmp::Ordering::Less if !empty => {}
            _ => return Err("check-run inventory is incomplete".into()),
        }
    }
    Err("check-run inventory exceeded its page limit".into())
}

pub fn merge(target: &Target, client: &Client, squash: &Squash, noun: &str) -> Result<(), String> {
    let input =
        runseal::github::Squash::new(&squash.candidate, Some(&squash.subject), Some(&squash.body))
            .map_err(diagnostic)?;
    let Patience { deadline, pause } = Patience::MERGE;
    let deadline = Instant::now() + deadline;
    loop {
        let view = client.readiness(target).map_err(diagnostic)?.value;
        if view.head != squash.candidate {
            return Err(format!(
                "{noun} pull head {} is not the exact candidate {}; merge refused",
                view.head, squash.candidate
            ));
        }
        let time = Instant::now() + pause < deadline;
        if view.state == "CLEAN" {
            let merged = submit(target, client, &input, time)?;
            if merged {
                return Ok(());
            }
        } else {
            readiness(noun, &view.state, time)?;
        }
        std::thread::sleep(pause);
    }
}

fn submit(
    target: &Target,
    client: &Client,
    input: &runseal::github::Squash,
    time: bool,
) -> Result<bool, String> {
    match client.squash(target, input) {
        Ok(_) => Ok(true),
        Err(error)
            if time && error.outcome == Outcome::Refused && error.evidence.status == Some(405) =>
        {
            Ok(false)
        }
        Err(error) => Err(diagnostic(error)),
    }
}

fn readiness(noun: &str, state: &str, time: bool) -> Result<(), String> {
    match state {
        "BLOCKED" | "UNKNOWN" if time => Ok(()),
        "BLOCKED" | "UNKNOWN" => Err(format!(
            "{noun} pull did not become mergeable within the bounded wait; last mergeStateStatus {state}"
        )),
        _ => Err(format!(
            "{noun} pull is not mergeable: mergeStateStatus {state}; merge refused"
        )),
    }
}
