use super::super::course::Course;
use super::super::value;
use super::plan;
use plumb::forgejo::{Client, Remote, Strategy, git};
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};
use std::time::Instant;

struct Published {
    version: String,
    commit: String,
    url: String,
}

struct Join<'a> {
    root: &'a Path,
    published: &'a Published,
    name: &'a str,
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
        joined(
            course,
            &client,
            Join {
                root,
                published: &published,
                name,
            },
        )?;
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

fn joined(course: &mut Course, client: &Client, held: Join<'_>) -> Result<(), String> {
    let Join {
        published, name, ..
    } = held;
    let remote = client.remote();
    let projection = format!("rejoin/{}", published.version);
    let made =
        format!("git commit-tree origin/main with {name} as second parent, then push {projection}");
    let head = course.step(made, || topology(&held, &projection))?;
    if course.dry() {
        course.step(
            format!(
                "POST /repos/{}/{}/pulls ({projection} -> main, fast-forward-only)",
                remote.owner, remote.repo
            ),
            || Ok(()),
        )?;
        course.step(
            "await guard on the topology-only merge, then fast-forward that pull",
            || Ok(()),
        )?;
        return Ok(());
    }
    let head = head.ok_or_else(|| "the settlement made no topology commit".to_string())?;
    let standing = client.find(&projection)?;
    let said = format!(
        "POST /repos/{}/{}/pulls ({projection} -> main, fast-forward-only)",
        remote.owner, remote.repo
    );
    let body = format!("Topology-preserving settlement of {}.", published.url);
    let pull = match standing {
        Some(pull) => Some(pull),
        None => course.step(said, || {
            client.raise(
                "main",
                &projection,
                &format!("Rejoin {}", published.version),
                &body,
            )
        })?,
    };
    let said = match &pull {
        Some(pull) => format!(
            "await guard on {head}, then fast-forward pull #{}",
            pull.number
        ),
        None => format!("await guard on {head}, then fast-forward that pull"),
    };
    course
        .step(said, || {
            let pull = pull.ok_or_else(|| "the settlement left no pull".to_string())?;
            guard(client, pull.number, &head)?;
            client.settle(pull.number, &head, Strategy::Forward)
        })
        .map(|_| ())
}

fn topology(held: &Join<'_>, projection: &str) -> Result<String, String> {
    let base = read(
        "resolve main for rejoin",
        command(held, &["rev-parse", "origin/main^{commit}"])?,
    )?;
    let tree = read(
        "resolve main tree for rejoin",
        command(held, &["rev-parse", "origin/main^{tree}"])?,
    )?;
    let message = format!("Rejoin {}", held.published.version);
    let head = read(
        "make topology-only rejoin",
        command(
            held,
            &[
                "commit-tree",
                &tree,
                "-p",
                &base,
                "-p",
                &held.published.commit,
                "-m",
                &message,
            ],
        )?,
    )?;
    value::commit(&head)?;
    success(
        "verify topology-only rejoin tree",
        command(held, &["diff-tree", "--quiet", &base, &head])?,
    )?;
    let refspec = format!("{head}:refs/heads/{projection}");
    success(
        "push topology-only rejoin",
        command(held, &["push", "--force-with-lease", "origin", &refspec])?,
    )?;
    Ok(head)
}

fn command(held: &Join<'_>, args: &[&str]) -> Result<Output, String> {
    Command::new("git")
        .args(args)
        .current_dir(held.root)
        .output()
        .map_err(|error| format!("cannot run git: {error}"))
}

fn success(action: &str, output: Output) -> Result<(), String> {
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "{action} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn read(action: &str, output: Output) -> Result<String, String> {
    success(action, output.clone())?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
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
    let deadline = Instant::now() + harness.guard.timeout.duration();
    while Instant::now() < deadline {
        let state = client.context(commit, "guard / guard (pull_request)")?;
        if state.count == 0 {
            std::thread::sleep(harness.guard.register.duration());
        } else if state.state == "success" {
            return Ok(());
        } else if state.state == "pending" {
            std::thread::sleep(harness.guard.pending.duration());
        } else {
            return Err(format!(
                "rejoin guard {} on {commit}; PR #{pull} left open",
                state.state
            ));
        }
    }
    Err(format!(
        "rejoin guard still pending on {commit} after {}s; PR #{pull} left open",
        harness.guard.timeout.seconds()
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
