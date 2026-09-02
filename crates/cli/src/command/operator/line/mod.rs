mod open;
mod projection;
mod recovery;
mod rejoined;
mod settle;

use super::super::version::Deed;
use super::course::Course;
use super::value;
use open::{freeze, opened};
use plumb::forgejo::{Client, Remote, git};
use serde_json::Value;
use settle::{Settle, settle};

pub fn run(deed: Deed) -> Result<String, String> {
    match deed {
        Deed::Prepare {
            version,
            from,
            repo,
            dry,
        } => prepare(&version, &from, &repo, dry),
        Deed::Pick {
            version,
            commit,
            dry,
        } => pick(&version, &commit, dry),
        Deed::Freeze { version, repo, dry } => wall(&version, &repo, dry),
        Deed::Rejoin { version, repo, dry } => rejoin(&version, &repo, dry),
    }
}

fn prepare(version: &str, from: &str, repo: &str, dry: bool) -> Result<String, String> {
    let version = value::version(version, "stable")?;
    let name = value::branch(&version);
    let root = git::root()?;
    let remote = git::remote(&root, repo)?;
    let mut course = Course::new(dry);
    git::fetch(&root)?;
    let seat = rejoined::Seat(&root);
    let activated = seat
        .marked(&version)?
        .then(|| {
            let spec = crate::shape::release::Spec::read(&root.join("plumb.toml"))?;
            super::super::release::Product::new(&spec).activated()
        })
        .transpose()?
        .flatten();
    seat.rejoined(activated)?;
    let client = Client::new(remote)?;
    let standing = client.branch(&name)?.is_some();
    let head = if standing {
        if super::mark::point(&root).seen(&version)?.is_some() {
            return Err(format!(
                "release marker {version} already stands; retract an unpublished marker before reopening {name}"
            ));
        }
        course.step(plan(client.remote(), &name, "preparing"), || {
            client.protect(&name, "preparing")
        })?;
        head(&client, &name)?
    } else {
        opened(&mut course, &client, &name, from)?
    };
    if !course.dry() {
        git::fetch(&root)?;
    }
    let head = course
        .step(super::version::plan(&version, &name), || {
            super::version::project(&root, &name, &version, &head)
        })?
        .unwrap_or(head);
    let recorded = course
        .step(super::datum::plan(&version), || {
            super::datum::record(super::datum::Cut {
                root: &root,
                name: &name,
                version: &version,
                head: &head,
            })
        })?
        .unwrap_or_default();
    if course.dry() {
        return Ok(course.plan());
    }
    if standing {
        Ok(format!(
            "{name} already stands at {head}; nothing moved; {recorded}"
        ))
    } else {
        Ok(format!("prepared {name} from {from} at {head}; {recorded}"))
    }
}

fn pick(version: &str, commits: &[String], dry: bool) -> Result<String, String> {
    let version = value::version(version, "stable")?;
    if commits.is_empty() {
        return Err("pick requires at least one commit".into());
    }
    for commit in commits {
        value::commit(commit)?;
    }
    let name = value::branch(&version);
    let mut course = Course::new(dry);
    let listed = commits.join(" ");
    let said = format!("git cherry-pick -x {listed} on {name}, then prove and push once");
    let done = course.step(said, || super::pick::pick(&git::root()?, &name, commits))?;
    if course.dry() {
        return Ok(course.plan());
    }
    done.ok_or_else(|| "the cherry-pick left no report".to_string())
}

fn wall(raw: &str, repo: &str, dry: bool) -> Result<String, String> {
    let version = value::version(raw, "stable")?;
    let name = value::branch(&version);
    let root = git::root()?;
    let remote = git::remote(&root, repo)?;
    let mut course = Course::new(dry);
    git::fetch(&root)?;
    let seat = rejoined::Seat(&root);
    let activated = seat
        .marked(&version)?
        .then(|| {
            let spec = crate::shape::release::Spec::read(&root.join("plumb.toml"))?;
            super::super::release::Product::new(&spec).activated()
        })
        .transpose()?
        .flatten();
    seat.rejoined(activated)?;
    let client = Client::new(remote)?;
    freeze(&mut course, &client, &root, &name)?;
    let spec = crate::shape::release::Spec::read(&root.join("plumb.toml"))?;
    let commit = head(&client, &name)?;
    let exact = super::super::release::Product::new(&spec).promotion(&commit, &version)?;
    if course.dry() {
        return Ok(course.plan());
    }
    Ok(format!(
        "froze {name} at {commit}; promotion source is {}",
        exact.version
    ))
}

fn head(client: &Client, name: &str) -> Result<String, String> {
    let branch = client.branch(name)?;
    let commit = branch
        .as_ref()
        .and_then(|held| held.pointer("/commit/id"))
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{name} exposes no head commit"))?;
    value::commit(commit)?;
    Ok(commit.to_string())
}

fn rejoin(version: &str, repo: &str, dry: bool) -> Result<String, String> {
    let version = value::version(version, "stable")?;
    let name = value::branch(&version);
    let root = git::root()?;
    let remote = git::remote(&root, repo)?;
    let mut course = Course::new(dry);
    let said = settle(
        &mut course,
        Settle {
            root: &root,
            remote,
            version: &version,
            name: &name,
        },
    )?;
    if course.dry() {
        return Ok(course.plan());
    }
    Ok(said)
}

fn plan(remote: &Remote, name: &str, mode: &str) -> String {
    format!(
        "PUT /repos/{}/{}/branch_protections/{name} ({mode})",
        remote.owner, remote.repo
    )
}
