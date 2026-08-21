use super::super::course::Course;
use super::super::value;
use super::plan;
use plumb::forgejo::{Client, Remote, git};
use serde_json::Value;
use std::path::Path;
use std::time::{Duration, Instant};

struct Published {
    version: String,
    commit: String,
    url: String,
}

pub struct Settle<'a> {
    pub root: &'a Path,
    pub remote: Remote,
    pub version: &'a str,
    pub name: &'a str,
}

pub fn settle(course: &mut Course, held: Settle<'_>) -> Result<String, String> {
    let Settle {
        root,
        remote,
        version,
        name,
    } = held;
    let published = pointer(root, version)?;
    git::fetch(root)?;
    let client = Client::new(remote)?;
    let branch = client.branch(name)?;
    let current = branch
        .as_ref()
        .and_then(|held| held.pointer("/commit/id"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if current != published.commit {
        return Err(format!(
            "{name} is {}, not published stable commit {}",
            if current.is_empty() {
                "absent"
            } else {
                current
            },
            published.commit
        ));
    }
    course.step(plan(client.remote(), name, "frozen"), || {
        client.protect(name, "frozen")
    })?;
    if !settled(root, &published)? {
        joined(course, &client, &published, name)?;
        if course.dry() {
            return Ok(String::new());
        }
        git::fetch(root)?;
        if !settled(root, &published)? {
            return Err(format!(
                "rejoin merge did not settle {version} into origin/main"
            ));
        }
    }
    Ok(format!(
        "rejoined {version} at {} into main; retained frozen {name}",
        published.commit
    ))
}

fn joined(
    course: &mut Course,
    client: &Client,
    published: &Published,
    name: &str,
) -> Result<(), String> {
    let remote = client.remote();
    let standing = client.find(name)?;
    let said = format!(
        "POST /repos/{}/{}/pulls ({name} -> main, merge commit)",
        remote.owner, remote.repo
    );
    let body = format!("Topology-preserving settlement of {}.", published.url);
    let pull = match standing {
        Some(pull) => Some(pull),
        None => course.step(said, || client.pull(name, &published.version, &body))?,
    };
    let commit = &published.commit;
    let said = match &pull {
        Some(pull) => format!("await guard on {commit}, then merge pull #{}", pull.number),
        None => format!("await guard on {commit}, then merge that pull"),
    };
    course
        .step(said, || {
            let pull = pull.ok_or_else(|| "the settlement left no pull".to_string())?;
            guard(client, pull.number, commit)?;
            client.merge(pull.number, commit)
        })
        .map(|_| ())
}

fn settled(root: &Path, published: &Published) -> Result<bool, String> {
    match super::super::super::release::settled(root, &published.version, &published.commit) {
        Ok(_) => Ok(true),
        Err(error) if error.contains("is not an ancestor") => Ok(false),
        Err(error) => Err(error),
    }
}

fn guard(client: &Client, pull: u64, commit: &str) -> Result<(), String> {
    let harness = plumb::forgejo::harness()?;
    let deadline = Instant::now() + Duration::from_millis(harness.guard_timeout_ms);
    while Instant::now() < deadline {
        let state = client.context(commit, "guard / guard (pull_request)")?;
        if state.count == 0 {
            std::thread::sleep(Duration::from_millis(harness.guard_register_ms));
        } else if state.state == "success" {
            return Ok(());
        } else if state.state == "pending" {
            std::thread::sleep(Duration::from_millis(harness.guard_pending_ms));
        } else {
            return Err(format!(
                "rejoin guard {} on {commit}; PR #{pull} left open",
                state.state
            ));
        }
    }
    Err(format!(
        "rejoin guard still pending on {commit} after {}s; PR #{pull} left open",
        harness.guard_timeout_ms / 1000
    ))
}

fn pointer(root: &Path, version: &str) -> Result<Published, String> {
    let authority = super::super::super::release::authority(root)?;
    let url = format!("{authority}/v1/channels/stable.json");
    super::super::super::release::inspect(&url)?;
    let value = plumb::forgejo::public(&url)?;
    if value.get("schema").and_then(Value::as_u64) != Some(1)
        || value.get("channel").and_then(Value::as_str) != Some("stable")
        || value.get("releaseVersion").and_then(Value::as_str) != Some(version)
    {
        return Err(format!("stable pointer does not name {version}"));
    }
    let commit = value
        .get("commit")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("stable pointer has no commit for {version}"))?;
    value::commit(commit)
        .map_err(|_| format!("stable pointer has an invalid commit for {version}"))?;
    Ok(Published {
        version: version.to_string(),
        commit: commit.to_string(),
        url,
    })
}
