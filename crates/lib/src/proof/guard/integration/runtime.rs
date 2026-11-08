use super::{Action, Descriptor};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Runtime {
    pub schema: String,
    pub root: String,
    pub ok: bool,
    pub strength: String,
    pub boundary: String,
    pub commit: String,
    pub guard: Descriptor,
    pub digest: String,
}

#[derive(Serialize)]
struct Claim<'a> {
    schema: &'static str,
    strength: &'static str,
    boundary: &'static str,
    repository: &'a str,
    commit: &'a str,
    tree: &'a str,
    plumb: &'a str,
    depot: &'a str,
    platform: &'a str,
    actions: &'a [Action],
    proof: &'a str,
    resolution: &'a Option<crate::packages::Resolution>,
}

impl Runtime {
    pub const SCHEMA: &str = "plumb.guard-runtime/v1";

    pub fn parse(raw: &[u8]) -> Result<Self, String> {
        if raw.is_empty() || raw.len() > 4 * 1024 * 1024 {
            return Err("runtime Guard document exceeds bounds".into());
        }
        let held: Self = serde_json::from_slice(raw)
            .map_err(|_| "invalid runtime Guard document".to_string())?;
        held.validate()?;
        Ok(held)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != Self::SCHEMA || !self.ok {
            return Err("runtime Guard is not a successful supported observation".into());
        }
        if self.strength != "full" || self.boundary != "head" {
            return Err("runtime Guard is not a successful full/head observation".into());
        }
        if self.root.is_empty() || self.commit.len() != 40 {
            return Err("runtime Guard has no exact source identity".into());
        }
        super::hash(&self.commit, "runtime commit")?;
        self.guard.validate()?;
        if Self::seal(&self.commit, &self.guard)? != self.digest {
            return Err("runtime Guard digest differs from its exact claim".into());
        }
        Ok(())
    }

    pub fn seal(commit: &str, guard: &Descriptor) -> Result<String, String> {
        guard.validate()?;
        if commit.len() != 40 {
            return Err("runtime Guard requires an exact commit".into());
        }
        super::hash(commit, "runtime commit")?;
        let claim = Claim {
            schema: Self::SCHEMA,
            strength: "full",
            boundary: "head",
            repository: &guard.repository,
            commit,
            tree: &guard.tree,
            plumb: &guard.plumb,
            depot: &guard.depot,
            platform: &guard.platform,
            actions: &guard.actions,
            proof: &guard.digest,
            resolution: &guard.resolution,
        };
        serde_json::to_vec(&claim)
            .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
            .map_err(|error| format!("cannot seal runtime Guard: {error}"))
    }
}
