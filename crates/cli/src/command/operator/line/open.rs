use super::super::course::Course;
use super::plan;
use plumb::forgejo::Client;
use std::path::Path;

pub fn opened(
    course: &mut Course,
    client: &Client,
    name: &str,
    from: &str,
) -> Result<String, String> {
    let remote = client.remote();
    course.step(plan(remote, name, "preparing"), || {
        client.protect(name, "preparing")
    })?;
    let said = format!(
        "POST /repos/{}/{}/branches ({name} from {from})",
        remote.owner, remote.repo
    );
    Ok(course
        .step(said, || client.create(name, from))?
        .map(|held| held.commit().to_string())
        .unwrap_or_else(|| from.to_string()))
}

pub fn freeze(course: &mut Course, client: &Client, root: &Path, name: &str) -> Result<(), String> {
    if client.branch(name)?.is_none() {
        return Err(format!("branch does not exist: {name}"));
    }
    super::super::pick::validate(root, name)?;
    course
        .step(plan(client.remote(), name, "frozen"), || {
            client.protect(name, "frozen")
        })
        .map(|_| ())
}
