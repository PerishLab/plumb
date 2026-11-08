use super::{Reference, wire::Wire};
use crate::semantics::Source;
use crate::windtun::qualification::{Fault, Read};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha1::{Digest, Sha1};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    pub product: String,
    pub platform: String,
    pub medium: String,
    pub adoption: Reference,
    pub canary: Reference,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Requirements {
    pub schema: String,
    pub scopes: Vec<Scope>,
}

impl Requirements {
    pub const SCHEMA: &str = "wharf.release.semantics.requirements/v1";

    pub fn parse(raw: &[u8]) -> Result<Self, Fault> {
        let held: Self = Wire::parse(raw)?;
        Wire::require(
            held.schema == Self::SCHEMA && held.scopes.len() <= 1024,
            "unsupported or unbounded semantic requirements",
        )?;
        Wire::require(
            Wire::canonical(&held)? == raw,
            "semantic requirements are not canonical",
        )?;
        let mut identities = BTreeSet::new();
        for scope in &held.scopes {
            Wire::name(&scope.product)?;
            Wire::name(&scope.platform)?;
            Wire::name(&scope.medium)?;
            Wire::require(
                identities.insert((&scope.product, &scope.platform, &scope.medium)),
                "semantic activation scope repeats",
            )?;
            scope.adoption.validate(&scope.product)?;
            scope.canary.validate(&scope.product)?;
        }
        Ok(held)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Authority {
    pub requirements: Requirements,
    pub source: Source,
    pub repository: Value,
    pub path: String,
    pub blob: String,
    pub sha256: String,
}

pub(super) struct Reader<'a, R>(pub &'a R);

impl<R: Read> Reader<'_, R> {
    const PATH: &'static str = "resources/check/semantics.json";

    fn head(&self) -> Result<String, Fault> {
        let value: Value = Wire::document(self.0, &format!("{}/branches/main", Wire::WHARF))?;
        Wire::require(
            value["name"] == "main" && value["protected"] == true,
            "semantic requirements need protected main",
        )?;
        let head = Wire::text(&value["commit"]["sha"])?;
        Wire::hex(head, 40)?;
        Ok(head.into())
    }

    fn content(&self, commit: &str) -> Result<(Vec<u8>, String), Fault> {
        let url = format!("{}/contents/{}?ref={commit}", Wire::WHARF, Self::PATH);
        let value: Value = Wire::document(self.0, &url)?;
        Wire::require(
            value["type"] == "file" && value["path"] == Self::PATH && value["encoding"] == "base64",
            "semantic requirements source entry differs",
        )?;
        let size = value["size"]
            .as_u64()
            .filter(|size| *size > 0 && *size <= 256 * 1024)
            .ok_or_else(|| Fault::refused("semantic requirements source exceeds bounds"))?;
        let text = value["content"]
            .as_str()
            .filter(|text| !text.is_empty() && text.len() <= 512 * 1024)
            .ok_or_else(|| Fault::refused("encoded semantic source exceeds bounds"))?;
        let raw = STANDARD
            .decode(text.lines().collect::<String>())
            .map_err(|_| Fault::refused("invalid semantic source base64"))?;
        let blob = Wire::text(&value["sha"])?;
        Wire::hex(blob, 40)?;
        let mut hash = Sha1::new();
        hash.update(format!("blob {}\0", raw.len()).as_bytes());
        hash.update(&raw);
        Wire::require(
            raw.len() as u64 == size && format!("{:x}", hash.finalize()) == blob,
            "semantic source differs from its native blob",
        )?;
        Ok((raw, blob.into()))
    }

    pub fn current(&self) -> Result<Authority, Fault> {
        let repository = Wire::repository(self.0, "PerishLab/wharf")?;
        Wire::require(
            repository["id"] == 1374358737_u64
                && repository["node_id"] == "R_kgDOUesM0Q"
                && repository["default_branch"] == "main",
            "foreign semantic control repository",
        )?;
        let commit = self.head()?;
        let identity: Value =
            Wire::document(self.0, &format!("{}/git/commits/{commit}", Wire::WHARF))?;
        Wire::require(
            identity["sha"] == commit,
            "semantic deployment commit differs",
        )?;
        let tree = Wire::text(&identity["tree"]["sha"])?;
        Wire::hex(tree, 40)?;
        let (raw, blob) = self.content(&commit)?;
        let requirements = Requirements::parse(&raw)?;
        Wire::require(
            self.head()? == commit && Wire::repository(self.0, "PerishLab/wharf")? == repository,
            "semantic authority moved during observation",
        )?;
        Ok(Authority {
            requirements,
            source: Source {
                repository: "PerishLab/wharf".into(),
                commit,
                tree: tree.into(),
            },
            repository,
            path: Self::PATH.into(),
            blob,
            sha256: Wire::sha(&raw),
        })
    }
}
