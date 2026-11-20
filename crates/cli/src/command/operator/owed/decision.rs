use semver::Version;
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

pub(super) type Result<T> = std::result::Result<T, String>;

pub(super) fn require(ok: bool, reason: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(format!("specified successor: {reason}"))
    }
}

pub(super) fn hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Decision {
    schema: String,
    pub repository: String,
    #[serde(rename = "repository_id")]
    pub identity: u64,
    pub authority: String,
    pub predecessor: Previous,
    pub successor: Next,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Previous {
    pub marker: String,
    pub tag: String,
    pub commit: String,
    pub tree: String,
    #[serde(rename = "distribution_sha256")]
    pub receipt: String,
    #[serde(rename = "channel_sha256")]
    pub channel: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Next {
    pub marker: String,
    pub rc: String,
    pub commit: String,
    pub tree: String,
}

pub(super) fn stable(marker: &str) -> Result<Version> {
    let parsed = marker
        .strip_prefix('v')
        .and_then(|v| Version::parse(v).ok())
        .ok_or_else(|| "specified successor: invalid stable version".to_string())?;
    require(
        parsed.pre.is_empty() && parsed.build.is_empty() && format!("v{parsed}") == marker,
        "expected canonical stable version",
    )?;
    Ok(parsed)
}

impl Decision {
    pub fn check(&self, repo: &str, id: u64) -> Result<()> {
        let old = &self.predecessor;
        let next = &self.successor;
        require(
            self.schema == "plumb.release.successor/v1"
                && self.repository == repo
                && self.identity == id,
            "record schema or repository identity differs",
        )?;
        let before = stable(&old.marker)?;
        let after = stable(&next.marker)?;
        require(
            (before.major, before.minor) == (after.major, after.minor)
                && before.patch.checked_add(1) == Some(after.patch)
                && next.rc == format!("{}-rc.1", next.marker),
            "expected the immediate higher patch and its first RC",
        )?;
        require(
            [&old.tag, &old.commit, &old.tree, &next.commit, &next.tree]
                .into_iter()
                .all(|s| hex(s, 40))
                && [&old.receipt, &old.channel].into_iter().all(|s| hex(s, 64)),
            "record contains a malformed object identity",
        )?;
        let host = self.authority.strip_prefix("https://").unwrap_or("");
        require(
            !host.is_empty()
                && host.split('.').all(|p| {
                    !p.is_empty() && p.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
                }),
            "invalid approved release authority",
        )
    }

    pub fn receipt(&self, raw: &[u8]) -> Result<()> {
        let old = &self.predecessor;
        require(
            format!("{:x}", Sha256::digest(raw)) == old.receipt,
            "original distribution bytes differ",
        )?;
        let v: Value = serde_json::from_slice(raw).map_err(|e| e.to_string())?;
        require(
            v["schema"] == 1 && v["repository"] == self.repository,
            "original distribution repository differs",
        )?;
        require(
            v["marker"] == old.marker && v["commit"] == old.commit && v["tree"] == old.tree,
            "original distribution source differs",
        )?;
        let media = v["media"]
            .as_object()
            .ok_or("specified successor: original distribution has no media")?;
        require(
            v["state"] == "incomplete"
                && v["media"]["channel"] == "unreached"
                && media.values().any(|m| m == "published"),
            "predecessor is not a partially published incomplete stable",
        )
    }

    pub fn channel(&self, raw: &[u8], product: &str) -> Result<()> {
        require(
            format!("{:x}", Sha256::digest(raw)) == self.predecessor.channel,
            "stable channel changed",
        )?;
        let v: Value = serde_json::from_slice(raw).map_err(|e| e.to_string())?;
        let marker = v["releaseVersion"].as_str().unwrap_or("");
        require(
            v["schema"] == 1 && v["product"] == product && v["channel"] == "stable",
            "stable channel identity differs",
        )?;
        require(
            hex(v["commit"].as_str().unwrap_or(""), 40)
                && stable(marker)? < stable(&self.predecessor.marker)?,
            "stable channel source or order differs",
        )
    }
}
