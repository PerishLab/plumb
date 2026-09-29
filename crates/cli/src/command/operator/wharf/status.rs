use plumb::seat::release::{Authority, Distribution, Marker};

pub(in crate::command) fn status(marker: &str) -> Result<String, String> {
    let channel = super::super::super::release::channel(marker)?;
    let root = super::super::worktree::root()
        .map_err(|error| format!("ship status runs in the product's repository: {error}"))?;
    let authority = Authority::new(&super::super::super::release::authority(&root)?)?;
    let url = authority.distribution(&channel, marker);
    let named = Marker {
        channel,
        marker: marker.to_string(),
    };
    match authority.read(&named)? {
        Some(held) => Ok(describe(&held)),
        None => Ok(format!(
            "{marker} has no distribution record at {url}; plumb ship dispatch --marker {marker} writes one"
        )),
    }
}

pub fn verdict(record: Option<Distribution>, marker: &str, run: &str) -> Result<String, String> {
    let held =
        record.ok_or_else(|| format!("run {run} wrote no distribution record for {marker}"))?;
    if held.run.as_deref() != Some(run) {
        return Err(format!(
            "run {run} did not write {marker}'s distribution record"
        ));
    }
    if !held.complete() {
        return Err(format!("run {run} left {}", describe(&held)));
    }
    Ok(describe(&held))
}

pub fn describe(held: &Distribution) -> String {
    let state = if held.state.is_empty() {
        "unknown"
    } else {
        &held.state
    };
    let mut said = vec![format!(
        "{} {state} at {}",
        held.marker,
        &held.commit[..held.commit.len().min(12)]
    )];
    for (name, value) in &held.media {
        said.push(format!("  {name:<9} {value}"));
    }
    said.join("\n")
}
