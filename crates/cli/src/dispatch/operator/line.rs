use super::Stable;
use super::value;
use plumb::forge::{Client, Remote, git};
use serde_json::Value;
use std::path::Path;
use std::time::{Duration, Instant};

struct Published {
    version: String,
    commit: String,
    url: String,
}

pub fn run(deed: Stable) -> Result<String, String> {
    match deed {
        Stable::Prepare {
            version,
            from,
            repo,
            dry,
        } => prepare(&version, &from, &repo, dry),
        Stable::Pick {
            version,
            commit,
            dry,
        } => pick(&version, &commit, dry),
        Stable::Freeze { version, repo, dry } => wall(&version, &repo, dry),
        Stable::Packport { version, repo, dry } => packport(&version, &repo, dry),
    }
}

fn prepare(version: &str, from: &str, repo: &str, dry: bool) -> Result<String, String> {
    let version = value::version(version, "stable")?;
    let name = value::branch(&version);
    let root = git::root()?;
    let remote = git::remote(&root, repo)?;
    if dry {
        return Ok(format!(
            "{}\nPOST /repos/{}/{}/branches ({name} from {from})",
            plan(&remote, &name, "preparing"),
            remote.owner,
            remote.repo
        ));
    }
    let client = Client::new(remote)?;
    client.protect(&name, "preparing")?;
    client.create(&name, from)?;
    Ok(format!("prepared {name} from {from}"))
}

fn pick(version: &str, commit: &str, dry: bool) -> Result<String, String> {
    let version = value::version(version, "stable")?;
    value::commit(commit)?;
    let name = value::branch(&version);
    if dry {
        return Ok(format!("git cherry-pick -x {commit} on {name}, then push"));
    }
    super::pick::pick(&git::root()?, &name, commit)
}

fn wall(raw: &str, repo: &str, dry: bool) -> Result<String, String> {
    let version = value::version(raw, "stable")?;
    let name = value::branch(&version);
    let root = git::root()?;
    let remote = git::remote(&root, repo)?;
    if dry {
        return Ok(plan(&remote, &name, "frozen"));
    }
    freeze(&Client::new(remote)?, &root, &name)?;
    Ok(format!("froze {name}"))
}

pub fn freeze(client: &Client, root: &Path, name: &str) -> Result<(), String> {
    if client.branch(name)?.is_none() {
        return Err(format!("branch does not exist: {name}"));
    }
    super::pick::validate(root, name)?;
    client.protect(name, "frozen")
}

fn packport(version: &str, repo: &str, dry: bool) -> Result<String, String> {
    let version = value::version(version, "stable")?;
    let name = value::branch(&version);
    let root = git::root()?;
    let remote = git::remote(&root, repo)?;
    if dry {
        return Ok(format!(
            "GET <plumb authority>/v1/channels/stable.json (expect {version})\n\
             GET /repos/{}/{}/branches/{name} (expect stable commit)\n{}\n\
             POST /repos/{}/{}/pulls ({name} -> main, merge commit)\n\
             prove stable commit is an ancestor of origin/main\nretain frozen {name}",
            remote.owner,
            remote.repo,
            plan(&remote, &name, "frozen"),
            remote.owner,
            remote.repo
        ));
    }
    settle(&root, remote, &version, &name)
}

fn settle(root: &Path, remote: Remote, version: &str, name: &str) -> Result<String, String> {
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
    client.protect(name, "frozen")?;
    if !settled(root, &published)? {
        let pull = match client.find(name)? {
            Some(pull) => pull,
            None => client.pull(
                name,
                version,
                &format!("Topology-preserving settlement of {}.", published.url),
            )?,
        };
        guard(&client, pull.number, &published.commit)?;
        client.merge(pull.number, &published.commit)?;
        git::fetch(root)?;
        if !settled(root, &published)? {
            return Err(format!(
                "packport merge did not settle {version} into origin/main"
            ));
        }
    }
    Ok(format!(
        "packported {version} at {} into main; retained frozen {name}",
        published.commit
    ))
}

fn pointer(root: &Path, version: &str) -> Result<Published, String> {
    let authority = super::super::release::authority(root)?;
    let url = format!("{authority}/v1/channels/stable.json");
    super::super::release::inspect(&url)?;
    let value = plumb::forge::public(&url)?;
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

fn settled(root: &Path, published: &Published) -> Result<bool, String> {
    match super::super::release::settled(root, &published.version, &published.commit) {
        Ok(_) => Ok(true),
        Err(error) if error.contains("is not an ancestor") => Ok(false),
        Err(error) => Err(error),
    }
}

fn guard(client: &Client, pull: u64, commit: &str) -> Result<(), String> {
    let harness = plumb::forge::harness()?;
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
                "packport guard {} on {commit}; PR #{pull} left open",
                state.state
            ));
        }
    }
    Err(format!(
        "packport guard still pending on {commit} after {}s; PR #{pull} left open",
        harness.guard_timeout_ms / 1000
    ))
}

fn plan(remote: &Remote, name: &str, mode: &str) -> String {
    format!(
        "PUT /repos/{}/{}/branch_protections/{name} ({mode})",
        remote.owner, remote.repo
    )
}
