use super::Dispatch;
use super::api::Client;
use super::{git, line, value};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

pub fn run(options: Dispatch) -> Result<String, String> {
    let root = git::root()?;
    let remote = git::remote(&root, &options.repo)?;
    let version = value::version(&options.version, &options.channel)?;
    let (workflow, reference, inputs) = if options.channel == "stable" {
        if options.promotion_channel == "stable" {
            return Err("stable promotion requires an exact non-stable channel".into());
        }
        if !options.r#ref.is_empty() {
            return Err(format!(
                "stable ref is derived as {}; remove --ref",
                value::branch(&version)
            ));
        }
        let promotion = value::version(&options.promotion_version, &options.promotion_channel)?;
        (
            "release-stable.yml",
            value::branch(&version),
            json!({
                "version": version,
                "promotion_channel": options.promotion_channel,
                "promotion_version": promotion
            }),
        )
    } else {
        value::exact(&options.channel)?;
        (
            "release-exact.yml",
            if options.r#ref.is_empty() {
                "main".to_string()
            } else {
                options.r#ref.clone()
            },
            json!({"channel": options.channel, "version": version}),
        )
    };
    if options.dry {
        let wall = (options.channel == "stable").then(|| {
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
    if options.channel == "stable" {
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
    let harness = super::settings::harness()?;
    let deadline = Instant::now() + Duration::from_millis(harness.run_timeout_ms);
    while Instant::now() < deadline {
        let run = client.run(id)?;
        let status = run.get("status").and_then(Value::as_str).unwrap_or("");
        if status == "success" {
            return Ok(format!("run {id}: success"));
        }
        if ["failure", "cancelled", "skipped"].contains(&status) {
            let jobs = if status == "failure" {
                client.failures(id)?
            } else {
                Vec::new()
            };
            let detail = if jobs.is_empty() {
                String::new()
            } else {
                format!("; failed jobs: {}", jobs.join(", "))
            };
            return Err(format!("run {id}: {status}{detail}\n{url}"));
        }
        std::thread::sleep(Duration::from_millis(harness.run_poll_ms));
    }
    Err(format!(
        "run {id}: still running past the watch timeout\n{url}"
    ))
}
