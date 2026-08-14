use super::Dispatch;
use super::{line, value};
use plumb::forgejo::{Client, Outcome, git};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

const JOBS: usize = 4;
const LINES: usize = 12;

pub fn run(options: Dispatch) -> Result<String, String> {
    let root = git::root()?;
    let remote = git::remote(&root, &options.repo)?;
    let channel = super::super::release::channel(&options.version)?;
    let version = value::version(&options.version, &channel)?;
    let (workflow, reference, inputs) = if channel == "stable" {
        if options.promotion_channel == "stable" {
            return Err("stable promotion requires an exact non-stable channel".into());
        }
        let promotion = value::version(&options.promotion_version, &options.promotion_channel)?;
        (
            "release-stable.yml",
            value::branch(&version),
            json!({
                "promotion_channel": options.promotion_channel,
                "promotion_version": promotion
            }),
        )
    } else {
        (
            "release-exact.yml",
            format!("refs/tags/{version}"),
            json!({}),
        )
    };
    if options.dry {
        let wall = (channel == "stable").then(|| {
            format!(
                "PUT /repos/{}/{}/branch_protections/{} (frozen)\n",
                remote.owner, remote.repo, reference
            )
        });
        return Ok(format!(
            "{}POST /repos/{}/{}/actions/workflows/{workflow}/dispatches (ref={reference}, inputs={inputs})",
            wall.unwrap_or_default(),
            remote.owner,
            remote.repo
        ));
    }
    let client = Client::new(remote)?;
    if channel == "stable" {
        line::freeze(&client, &root, &reference)?;
    }
    let run = client.dispatch(workflow, &reference, inputs)?;
    let id = run
        .get("id")
        .and_then(Value::as_u64)
        .ok_or_else(|| "Forgejo did not expose the dispatched run ID".to_string())?;
    let url = client.link(&run);
    if url.is_empty() {
        return Err(format!(
            "Forgejo did not expose a canonical URL for run {id}"
        ));
    }
    let mut message = format!("triggered {workflow} run {id} for ref {reference}\n{url}");
    if options.watch {
        message.push('\n');
        message.push_str(&watch(&client, id, &url)?);
    }
    Ok(message)
}

fn watch(client: &Client, id: u64, url: &str) -> Result<String, String> {
    let harness = plumb::forgejo::harness()?;
    let deadline = Instant::now() + Duration::from_millis(harness.run_timeout_ms);
    while Instant::now() < deadline {
        match client.outcome(id)? {
            Outcome::Success => return Ok(format!("run {id}: success")),
            Outcome::Failed { status, tasks } => {
                return Err(report(client, id, &brief(id, url, &status, &tasks)));
            }
            Outcome::Waiting => {}
        }
        std::thread::sleep(Duration::from_millis(harness.run_poll_ms));
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
