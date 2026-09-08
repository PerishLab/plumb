use crate::command::release::{self, ReleaseMarker};
use plumb::forgejo::git;
use semver::Version;
use serde_json::Value;

pub(super) fn verify(marker: &ReleaseMarker) -> Result<(), String> {
    if !marker.independent() || marker.channel != "stable" {
        return Ok(());
    }
    let mut candidates = Vec::new();
    for tag in git::tags(&marker.spec().root, &marker.commit)? {
        let Ok(version) = Version::parse(tag.trim_start_matches('v')) else {
            continue;
        };
        if version.pre.is_empty() || tag.split('-').next() != Some(marker.base()) {
            continue;
        }
        candidates.push((version, tag));
    }
    candidates.sort_by(|left, right| right.0.cmp(&left.0));
    for (_, tag) in candidates {
        let candidate = release::snapshot(&tag)?;
        let identity = |held: &ReleaseMarker| {
            (
                held.product.clone(),
                held.commit.clone(),
                held.tree.clone(),
                held.profile.clone(),
            )
        };
        if identity(&candidate) != identity(marker) {
            continue;
        }
        if completed(&candidate)? {
            return Ok(());
        }
    }
    Err(format!(
        "stable ship {} requires a fully proven candidate publication graph at {}; release marker identity remains valid",
        marker.marker, marker.commit
    ))
}

pub(super) fn completed(marker: &ReleaseMarker) -> Result<bool, String> {
    let graph: Value = serde_json::from_str(&super::resolve::graph(marker, true)?)
        .map_err(|error| format!("cannot read ship proof: {error}"))?;
    Ok(graph["workload_missing"] == false && graph["publication_missing"] == false)
}
