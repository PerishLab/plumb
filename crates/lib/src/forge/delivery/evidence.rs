use super::{Context, refuse, squash};
use crate::landing::Refusal;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

pub const TRAILER: &str = "Native-Gate-Proof:";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub authority: String,
    pub source: String,
    pub tree: String,
    pub digest: String,
}

impl Evidence {
    pub(super) fn matches(&self, context: &Context<'_>) -> Result<(), Refusal> {
        self.validate()?;
        if self.source != context.source || self.tree != context.tree {
            return Err(refuse(
                "native",
                "native evidence does not bind the exact source and projected tree",
            ));
        }
        Ok(())
    }

    pub(super) fn encode(&self) -> Result<String, Refusal> {
        self.validate()?;
        serde_json::to_vec(self)
            .map(|bytes| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
            .map_err(|error| refuse("native", format!("cannot encode native evidence: {error}")))
    }

    fn validate(&self) -> Result<(), Refusal> {
        if self.authority.trim().is_empty() || self.authority != self.authority.trim() {
            return Err(refuse(
                "native",
                "native evidence requires an explicit authority",
            ));
        }
        if !hex(&self.source, 40) || !hex(&self.tree, 40) || !hex(&self.digest, 64) {
            return Err(refuse(
                "native",
                "native evidence requires exact source/tree IDs and a SHA-256 result digest",
            ));
        }
        Ok(())
    }
}

pub fn admit(root: &Path, commit: &str) -> Result<(), Refusal> {
    let output = Command::new("git")
        .args([
            "-C",
            root.to_str()
                .ok_or_else(|| refuse("root", "non-UTF8 repository path"))?,
            "ls-tree",
            commit,
            "--",
            "plumb.toml",
        ])
        .output()
        .map_err(|error| {
            refuse(
                "git",
                format!("cannot inspect governance declaration: {error}"),
            )
        })?;
    if !output.status.success() {
        return Err(refuse(
            "git",
            "cannot inspect native governance declaration",
        ));
    }
    let message = squash::git(root, &["log", "-1", "--format=%B", commit])?;
    if !output.stdout.is_empty()
        || message
            .lines()
            .any(|line| line.starts_with(crate::guard::TRAILER))
    {
        return Err(refuse(
            "native",
            "Plumb-governed source or base cannot use native gate delivery",
        ));
    }
    Ok(())
}

pub fn read(root: &Path, commit: &str) -> Result<Evidence, Refusal> {
    let message = squash::git(root, &["log", "-1", "--format=%B", commit])?;
    let tokens: Vec<_> = message
        .lines()
        .filter_map(|line| line.strip_prefix(TRAILER))
        .collect();
    if tokens.len() != 1
        || message
            .lines()
            .any(|line| line.starts_with(crate::guard::TRAILER))
    {
        return Err(refuse(
            "native",
            "candidate must carry exactly one native evidence trailer and no Plumb Guard trailer",
        ));
    }
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(tokens[0].trim())
        .map_err(|error| refuse("native", format!("cannot decode native evidence: {error}")))?;
    let evidence: Evidence = serde_json::from_slice(&bytes)
        .map_err(|error| refuse("native", format!("cannot parse native evidence: {error}")))?;
    evidence.validate()?;
    Ok(evidence)
}

fn hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
