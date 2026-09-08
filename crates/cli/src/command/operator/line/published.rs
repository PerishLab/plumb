use crate::command::{release, ship};
use serde_json::Value;
use std::path::Path;

pub(super) struct Published {
    pub version: String,
    pub commit: String,
    pub url: String,
}

pub(super) fn read(root: &Path, version: &str) -> Result<Published, String> {
    if let Some(marker) = release::ReleaseMarker::recorded(root, version)? {
        if marker.channel != "stable" || !ship::completed(&marker)? {
            return Err(format!(
                "version rejoin requires a complete stable Ship graph for {version}"
            ));
        }
        let remote = plumb::forgejo::git::remote(root, "")?;
        return Ok(Published {
            version: marker.version,
            commit: marker.commit,
            url: format!(
                "https://{}/{}/{}/src/tag/{version}",
                remote.host, remote.owner, remote.repo
            ),
        });
    }
    pointer(root, version)
}

fn pointer(root: &Path, version: &str) -> Result<Published, String> {
    let authority = release::authority(root)?;
    let url = format!("{authority}/v1/channels/stable.json");
    release::inspect(&url)?;
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
    super::super::value::commit(commit)
        .map_err(|_| format!("stable pointer has an invalid commit for {version}"))?;
    Ok(Published {
        version: version.to_string(),
        commit: commit.to_string(),
        url,
    })
}
