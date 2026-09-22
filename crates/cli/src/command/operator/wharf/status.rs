use super::super::owed::distribution;
use serde_json::Value;

pub(in crate::command) fn status(marker: &str) -> Result<String, String> {
    let channel = super::super::super::release::channel(marker)?;
    let root = super::super::worktree::root()
        .map_err(|error| format!("ship status runs in the product's repository: {error}"))?;
    let authority = super::super::super::release::authority(&root)?;
    let url = distribution(&authority, &channel, marker);
    match read(&url)? {
        Some(body) => Ok(describe(&record(&body, marker)?)),
        None => Ok(format!(
            "{marker} has no distribution record at {url}; plumb ship dispatch --marker {marker} writes one"
        )),
    }
}

pub fn read(url: &str) -> Result<Option<Vec<u8>>, String> {
    plumb::bucket::fetch(url).map_err(|error| format!("cannot read {url}: {error}"))
}

pub fn verdict(body: Option<Vec<u8>>, marker: &str, run: &str) -> Result<String, String> {
    let body =
        body.ok_or_else(|| format!("run {run} wrote no distribution record for {marker}"))?;
    let held = record(&body, marker)?;
    if held["attempt"]["run"] != run {
        return Err(format!(
            "run {run} did not write {marker}'s distribution record"
        ));
    }
    if held["state"] != "complete" {
        return Err(format!("run {run} left {}", describe(&held)));
    }
    Ok(describe(&held))
}

pub fn record(body: &[u8], marker: &str) -> Result<Value, String> {
    let held: Value = serde_json::from_slice(body)
        .map_err(|error| format!("{marker} distribution record does not parse: {error}"))?;
    if held["marker"] != marker {
        return Err(format!(
            "{marker} distribution record names {}",
            held["marker"]
        ));
    }
    Ok(held)
}

pub fn describe(held: &Value) -> String {
    let commit = held["commit"].as_str().unwrap_or_default();
    let mut said = vec![format!(
        "{} {} at {}",
        held["marker"].as_str().unwrap_or_default(),
        held["state"].as_str().unwrap_or("unknown"),
        &commit[..commit.len().min(12)]
    )];
    if let Some(media) = held["media"].as_object() {
        for (name, state) in media {
            said.push(format!(
                "  {name:<9} {}",
                state.as_str().unwrap_or_default()
            ));
        }
    }
    said.join("\n")
}

pub fn public(held: &Value) -> Vec<String> {
    const PUBLIC: [&str; 5] = ["published", "deployed", "pointed", "present", "skipped"];
    held["media"]
        .as_object()
        .map(|media| {
            media
                .iter()
                .filter(|(_, state)| state.as_str().is_some_and(|state| PUBLIC.contains(&state)))
                .map(|(name, _)| name.clone())
                .collect()
        })
        .unwrap_or_default()
}
