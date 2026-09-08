use super::super::course::Course;
use super::plan;
use super::published::{Published, read};
use plumb::forgejo::{Client, Pull, Remote, Strategy, git};
use serde_json::Value;
use std::path::Path;

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
    git::fetch(root)?;
    let published = read(root, version)?;
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
    let standing = client.find(&projection)?;
    let (head, pull) = match standing {
        Some(pull) => {
            let (head, pull) = resume(course, &held, &projection, pull)?;
            let Some(head) = head else {
                return Ok(());
            };
            (head, Some(pull))
        }
        None => {
            let made = format!(
                "git commit-tree origin/main with {name} as second parent, then push {projection}"
            );
            let head = course.step(made, || project(&held, &projection))?;
            if course.dry() {
                course.step(
                    format!(
                        "POST /repos/{}/{}/pulls ({projection} -> main, fast-forward-only)",
                        remote.owner, remote.repo
                    ),
                    || Ok(()),
                )?;
                course.step("attest the inherited main tree Guard proof", || Ok(()))?;
                course.step("fast-forward the topology-only pull", || Ok(()))?;
                return Ok(());
            }
            let head = head.ok_or_else(|| "the settlement made no topology commit".to_string())?;
            let said = format!(
                "POST /repos/{}/{}/pulls ({projection} -> main, fast-forward-only)",
                remote.owner, remote.repo
            );
            let body = format!("Topology-preserving settlement of {}.", published.url);
            let pull = course.step(said, || {
                client.raise(
                    "main",
                    &projection,
                    &format!("Rejoin {}", published.version),
                    &body,
                )
            })?;
            (head, pull)
        }
    };
    if course.dry() {
        course.step("attest the inherited main tree Guard proof", || Ok(()))?;
        course.step("fast-forward the existing topology-only pull", || Ok(()))?;
        return Ok(());
    }
    course.step("attest the inherited main tree Guard proof", || {
        super::projection::proved(held.root, &head)?;
        client.mark(
            &head,
            "guard / guard (pull_request)",
            "Plumb verified rejoin preserves main's guarded tree",
        )
    })?;
    let said = match &pull {
        Some(pull) => format!("fast-forward topology-only pull #{} at {head}", pull.number),
        None => format!("fast-forward the topology-only pull at {head}"),
    };
    course
        .step(said, || {
            let pull = pull.ok_or_else(|| "the settlement left no pull".to_string())?;
            client.settle(pull.number, &head, Strategy::Forward)
        })
        .map(|_| ())
}

fn resume(
    course: &mut Course,
    held: &Join<'_>,
    projection: &str,
    pull: Pull,
) -> Result<(Option<String>, Pull), String> {
    match super::recovery::head(held.root, &held.published.commit, projection)? {
        super::recovery::Head::Current(head) => {
            course.step(
                format!("reuse open pull #{} at {head}", pull.number),
                || Ok(()),
            )?;
            Ok((Some(head), pull))
        }
        super::recovery::Head::Stale => {
            let made = format!(
                "retarget open pull #{} onto current main while preserving {}",
                pull.number, held.name
            );
            let head = course.step(made, || project(held, projection))?;
            if course.dry() {
                course.step("attest the inherited main tree Guard proof", || Ok(()))?;
                course.step("fast-forward the retargeted topology-only pull", || Ok(()))?;
                return Ok((None, pull));
            }
            let head = head.ok_or_else(|| {
                "the settlement did not retarget the stale topology commit".to_string()
            })?;
            Ok((Some(head), pull))
        }
    }
}

fn project(held: &Join<'_>, projection: &str) -> Result<String, String> {
    super::projection::make(
        held.root,
        &held.published.version,
        &held.published.commit,
        projection,
    )
}

fn settled(root: &Path, published: &Published) -> Result<bool, String> {
    match super::super::super::release::settled(root, &published.version, &published.commit) {
        Ok(_) => Ok(true),
        Err(error) if error.contains("is not an ancestor") => Ok(false),
        Err(error) => Err(error),
    }
}
