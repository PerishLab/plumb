use crate::windtun::qualification::{Fault, Read};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub(super) struct Wire;

impl Wire {
    pub const LIMIT: usize = 4 * 1024 * 1024;
    pub const WHARF: &str = "https://api.github.com/repos/PerishLab/wharf";

    pub fn require(ok: bool, reason: &str) -> Result<(), Fault> {
        if ok {
            Ok(())
        } else {
            Err(Fault::refused(reason))
        }
    }

    pub fn sha(raw: &[u8]) -> String {
        format!("{:x}", Sha256::digest(raw))
    }

    pub fn name(value: &str) -> Result<(), Fault> {
        super::super::wire::Wire::name(value).map_err(Fault::refused)
    }

    pub fn hex(value: &str, size: usize) -> Result<(), Fault> {
        super::super::wire::Wire::hex(value, size).map_err(Fault::refused)
    }

    pub fn text(value: &Value) -> Result<&str, Fault> {
        value
            .as_str()
            .filter(|text| !text.is_empty() && text.len() <= 8192)
            .ok_or_else(|| Fault::refused("native text identity is absent or unbounded"))
    }

    pub fn document<T: DeserializeOwned>(reader: &impl Read, url: &str) -> Result<T, Fault> {
        let raw = reader.bytes(url, Self::LIMIT)?;
        Self::parse(&raw)
    }

    pub fn parse<T: DeserializeOwned>(raw: &[u8]) -> Result<T, Fault> {
        Self::require(
            !raw.is_empty() && raw.len() <= Self::LIMIT,
            "semantic document exceeds bounds",
        )?;
        serde_json::from_slice(raw).map_err(|_| Fault::refused("invalid semantic document"))
    }

    pub fn canonical(value: &impl Serialize) -> Result<Vec<u8>, Fault> {
        let mut held = serde_json::to_value(value)
            .map_err(|_| Fault::refused("cannot encode semantic document"))?;
        held.sort_all_objects();
        serde_json::to_vec(&held)
            .map_err(|_| Fault::refused("cannot encode canonical semantic document"))
    }

    pub fn marked<T: DeserializeOwned>(body: &str, schema: &str) -> Result<T, Fault> {
        Self::require(
            !body.is_empty() && body.len() <= 64 * 1024,
            "semantic review exceeds bounds",
        )?;
        let start = format!("<!-- {schema}\n");
        Self::require(
            body.matches(&start).count() == 1,
            "semantic review requires exactly one supported marker",
        )?;
        let (_, tail) = body.split_once(&start).unwrap();
        let (raw, _) = tail
            .split_once("\n-->")
            .ok_or_else(|| Fault::refused("semantic review marker is incomplete"))?;
        Self::parse(raw.as_bytes())
    }

    pub fn repository(reader: &impl Read, name: &str) -> Result<Value, Fault> {
        let product = name
            .strip_prefix("PerishLab/")
            .ok_or_else(|| Fault::refused("semantic subject requires PerishLab"))?;
        if product != ".github" {
            Wire::name(product)?;
        }
        let value: Value = Self::document(reader, &format!("https://api.github.com/repos/{name}"))?;
        Self::require(
            value["full_name"] == name && value["visibility"] == "public",
            "semantic repository authority differs",
        )?;
        Self::require(
            value["archived"] == false && value["disabled"] == false,
            "semantic repository is inactive",
        )?;
        Self::require(
            value["owner"]["id"] == 330236468_u64
                && value["owner"]["node_id"] == "O_kgDOE68CNA"
                && value["owner"]["login"] == "PerishLab",
            "semantic repository owner differs",
        )?;
        Self::require(
            value["id"].as_u64().is_some_and(|id| id > 0),
            "semantic repository numeric identity is absent",
        )?;
        Self::text(&value["node_id"])?;
        Ok(
            json!({"id":value["id"], "node_id":value["node_id"], "full_name":name,
            "default_branch":value["default_branch"], "owner":{"id":value["owner"]["id"],
            "node_id":value["owner"]["node_id"], "login":value["owner"]["login"]}}),
        )
    }
}
