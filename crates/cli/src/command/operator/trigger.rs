use super::Dispatch;
use super::course::Course;
use plumb::forgejo::{Client, Outcome, git};
use serde_json::{Value, json};
use std::io::Write as _;
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
    let configuration = plumb::version!("PLUMB").to_string();
    let root = git::root()?;
    let product = git::remote(&root, &options.repo)?;
    let remote = git::remote(&root, "PerishLab/plumb")?;
    let workflow = "ship.yml";
    let reference = atom(
        plumb::commit!("PLUMB"),
        plumb::version!("PLUMB").trim_start_matches('v'),
    );
    let client = Client::new(remote)?;
    let mut course = Course::new(options.dry);
    let message = launch(
        Flight {
            client: &client,
            workflow,
            reference: &reference,
            inputs: json!({
                "configuration": configuration,
                "marker": before.marker,
                "plumb": plumb::version!("PLUMB"),
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

fn atom(commit: Option<&str>, version: &str) -> String {
    commit.map_or_else(
        || format!("refs/tags/v{version}"),
        std::string::ToString::to_string,
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
    let number = run
        .get("run_number")
        .and_then(Value::as_u64)
        .ok_or_else(|| "Forgejo did not expose the dispatched run number".to_string())?;
    let url = flight.client.link(&run);
    if url.is_empty() {
        return Err(format!(
            "Forgejo did not expose a canonical URL for run {id}"
        ));
    }
    let message = format!(
        "triggered {} run {id} for ref {}\n{url}",
        flight.workflow, flight.reference
    );
    if flight.waiting {
        println!("{message}");
        std::io::stdout()
            .flush()
            .map_err(|error| format!("cannot report dispatched run {id}: {error}"))?;
        return watch(flight.client, id, number, &url);
    }
    Ok(message)
}

fn watch(client: &Client, id: u64, number: u64, url: &str) -> Result<String, String> {
    let harness = plumb::forgejo::harness()?;
    let deadline = Instant::now() + harness.run.timeout.duration();
    let mut settled = 0;
    while Instant::now() < deadline {
        match client.outcome(number)? {
            Outcome::Success if settle(&mut settled) => return Ok(format!("run {id}: success")),
            Outcome::Success => {}
            Outcome::Failed { status, tasks } => {
                return Err(report(client, number, &brief(id, url, &status, &tasks)));
            }
            Outcome::Waiting => settled = 0,
        }
        std::thread::sleep(harness.run.poll.duration());
    }
    Err(format!(
        "run {id}: still running past the watch timeout\n{url}"
    ))
}

fn settle(held: &mut usize) -> bool {
    *held += 1;
    *held == 3
}

fn brief(id: u64, url: &str, status: &str, tasks: &[String]) -> String {
    let detail = tasks
        .is_empty()
        .then(String::new)
        .unwrap_or_else(|| format!("; failed tasks: {}", tasks.join(", ")));
    format!("run {id}: {status}{detail}\n{url}")
}

fn report(client: &Client, number: u64, brief: &str) -> String {
    match client.logs(number) {
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
