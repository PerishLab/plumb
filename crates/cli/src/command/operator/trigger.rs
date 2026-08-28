use super::course::Course;
use super::{Dispatch, Legacy};
use super::{line, value};
use plumb::forgejo::{Client, Outcome, git};
use serde_json::{Value, json};
use std::time::Instant;

const JOBS: usize = 4;
const LINES: usize = 12;

struct Flight<'a> {
    client: &'a Client,
    workflow: &'a str,
    reference: &'a str,
    inputs: Value,
    waiting: bool,
}

pub fn run(options: Dispatch) -> Result<String, String> {
    let before = super::super::release::marker(&options.marker)?;
    let digest = before.digest()?;
    let root = git::root()?;
    let product = git::remote(&root, &options.repo)?;
    let remote = git::remote(&root, "PerishLab/plumb")?;
    let workflow = "ship.yml";
    let reference = if before.repository == "PerishLab/plumb" {
        format!("refs/tags/{}", before.marker)
    } else {
        format!("refs/tags/v{}", env!("CARGO_PKG_VERSION"))
    };
    let client = Client::new(remote)?;
    let mut course = Course::new(options.dry);
    let message = launch(
        Flight {
            client: &client,
            workflow,
            reference: &reference,
            inputs: json!({
                "marker": before.marker,
                "repository": format!("{}/{}", product.owner, product.repo),
            }),
            waiting: options.watch,
        },
        &mut course,
    );
    let after = super::super::release::marker(&options.marker)?;
    if after.digest()? != digest {
        return Err(format!(
            "release marker {} drifted while ship was in flight",
            after.marker
        ));
    }
    message
}

pub fn legacy(options: Legacy) -> Result<String, String> {
    let root = git::root()?;
    current(&root)?;
    let remote = git::remote(&root, &options.repo)?;
    let channel = super::super::release::channel(&options.version)?;
    let version = value::version(&options.version, &channel)?;
    let (workflow, reference, inputs) = if channel == "stable" {
        ("stable.release.yml", value::branch(&version), json!({}))
    } else {
        (
            "exact.release.yml",
            format!("refs/tags/{version}"),
            json!({}),
        )
    };
    present(&root, workflow)?;
    let mut course = Course::new(options.dry);
    let client = Client::new(remote)?;
    if channel == "stable" {
        line::freeze(&mut course, &client, &root, &reference)?;
        super::mark::stood(&root, &version, &reference)?;
    }
    launch(
        Flight {
            client: &client,
            workflow,
            reference: &reference,
            inputs,
            waiting: options.watch,
        },
        &mut course,
    )
}

fn launch(flight: Flight<'_>, course: &mut Course) -> Result<String, String> {
    let said = format!(
        "POST /repos/{}/{}/actions/workflows/{}/dispatches (ref={}, inputs={})",
        flight.client.remote().owner,
        flight.client.remote().repo,
        flight.workflow,
        flight.reference,
        flight.inputs
    );
    let run = course.step(said, || {
        flight
            .client
            .dispatch(flight.workflow, flight.reference, flight.inputs)
    })?;
    if course.dry() {
        return Ok(course.plan());
    }
    let run = run.ok_or_else(|| "the dispatch left no run".to_string())?;
    let id = run
        .get("id")
        .and_then(Value::as_u64)
        .ok_or_else(|| "Forgejo did not expose the dispatched run ID".to_string())?;
    let url = flight.client.link(&run);
    if url.is_empty() {
        return Err(format!(
            "Forgejo did not expose a canonical URL for run {id}"
        ));
    }
    let mut message = format!(
        "triggered {} run {id} for ref {}\n{url}",
        flight.workflow, flight.reference
    );
    if flight.waiting {
        message.push('\n');
        message.push_str(&watch(flight.client, id, &url)?);
    }
    Ok(message)
}

fn current(root: &std::path::Path) -> Result<(), String> {
    let stale = crate::command::lane::Seat(root).stale();
    if stale.is_empty() {
        return Ok(());
    }
    Err(format!(
        "a release cannot start on a lane this Plumb did not render: {}; run plumb lane --write",
        stale.join(", ")
    ))
}

fn present(root: &std::path::Path, workflow: &str) -> Result<(), String> {
    let path = format!(".forgejo/workflows/{workflow}");
    if !root.join(&path).is_file() {
        return Err(format!(
            "a release cannot start without its legacy workflow: {path}"
        ));
    }
    let Ok(evidence) = crate::command::lane::Seat(root).project() else {
        return Ok(());
    };
    if evidence
        .projected()
        .iter()
        .any(|lane| lane.path == path && lane.absent())
    {
        return Err(format!(
            "a release cannot start on a lane this repository has not rendered: {path}; run plumb lane --write"
        ));
    }
    Ok(())
}

fn watch(client: &Client, id: u64, url: &str) -> Result<String, String> {
    let harness = plumb::forgejo::harness()?;
    let deadline = Instant::now() + harness.run.timeout.duration();
    while Instant::now() < deadline {
        match client.outcome(id)? {
            Outcome::Success => return Ok(format!("run {id}: success")),
            Outcome::Failed { status, tasks } => {
                return Err(report(client, id, &brief(id, url, &status, &tasks)));
            }
            Outcome::Waiting => {}
        }
        std::thread::sleep(harness.run.poll.duration());
    }
    Err(format!(
        "run {id}: still running past the watch timeout\n{url}"
    ))
}

fn brief(id: u64, url: &str, status: &str, tasks: &[String]) -> String {
    let detail = tasks
        .is_empty()
        .then(String::new)
        .unwrap_or_else(|| format!("; failed tasks: {}", tasks.join(", ")));
    format!("run {id}: {status}{detail}\n{url}")
}

fn report(client: &Client, id: u64, brief: &str) -> String {
    match client.logs(id) {
        Ok(found) if !found.is_empty() => format!("{brief}\n{}", tails(&found)),
        _ => brief.to_string(),
    }
}

fn tails(found: &[(usize, String)]) -> String {
    let refused: Vec<&(usize, String)> = found
        .iter()
        .filter(|(_, text)| text.contains("Job failed"))
        .collect();
    let chosen = if refused.is_empty() {
        found.iter().collect()
    } else {
        refused
    };
    chosen
        .iter()
        .take(JOBS)
        .map(|(job, text)| {
            let lines: Vec<&str> = text.lines().collect();
            let tail = lines[lines.len().saturating_sub(LINES)..]
                .iter()
                .map(|line| format!("  {}", stamp(line)))
                .collect::<Vec<_>>()
                .join("\n");
            format!("job {job}:\n{tail}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn stamp(line: &str) -> &str {
    line.split_once(' ').map_or(line, |(head, rest)| {
        if head.ends_with('Z') && head.contains('T') {
            rest
        } else {
            line
        }
    })
}
