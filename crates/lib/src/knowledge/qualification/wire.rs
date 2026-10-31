use super::{Fault, Read, Result, unique::Unique};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub(super) const LIMIT: usize = 4 * 1024 * 1024;
pub(super) const ARCHIVE: usize = 64 * 1024 * 1024;
pub(super) const POLICY: &str = "wharf.windtun.linux/v1";
pub(super) const WHARF: &str = "https://api.github.com/repos/PerishLab/wharf";

pub(super) struct Wire;

impl Wire {
    pub fn require(ok: bool, reason: &str) -> Result<()> {
        if ok {
            Ok(())
        } else {
            Err(Fault::refused(reason))
        }
    }

    pub fn parse(raw: &[u8], canonical: bool) -> Result<Value> {
        Self::require(
            !raw.is_empty() && raw.len() <= LIMIT,
            "JSON material exceeds bounds",
        )?;
        let value: Unique = serde_json::from_slice(raw)
            .map_err(|_| Fault::refused("invalid or duplicate JSON fields"))?;
        if canonical {
            Self::require(
                Self::encode(&value.0)? == raw,
                "protocol JSON is not canonical",
            )?;
        }
        Ok(value.0)
    }

    pub fn encode(value: &Value) -> Result<Vec<u8>> {
        serde_json::to_vec(&Self::sorted(value))
            .map_err(|_| Fault::refused("JSON cannot be encoded"))
    }

    fn sorted(value: &Value) -> Value {
        match value {
            Value::Object(fields) => {
                let mut names: Vec<_> = fields.keys().collect();
                names.sort();
                Value::Object(
                    names
                        .into_iter()
                        .map(|name| (name.clone(), Self::sorted(&fields[name])))
                        .collect(),
                )
            }
            Value::Array(values) => Value::Array(values.iter().map(Self::sorted).collect()),
            other => other.clone(),
        }
    }

    pub fn sha(raw: &[u8]) -> String {
        format!("{:x}", Sha256::digest(raw))
    }

    pub fn digest(value: &Value) -> Result<String> {
        Ok(Self::sha(&Self::encode(value)?))
    }

    pub fn fields(value: &Value, fields: &[&str]) -> Result<()> {
        let object = value
            .as_object()
            .ok_or_else(|| Fault::refused("expected an object"))?;
        Self::require(
            object.len() == fields.len() && fields.iter().all(|name| object.contains_key(*name)),
            "protocol fields differ",
        )
    }

    pub fn matches(value: &Value, model: &Value, reason: &str) -> Result<()> {
        let fields = model
            .as_object()
            .ok_or_else(|| Fault::refused("invalid identity model"))?;
        Self::require(
            fields
                .iter()
                .all(|(key, expected)| value.get(key) == Some(expected)),
            reason,
        )
    }

    pub fn text(value: &Value) -> Result<&str> {
        value
            .as_str()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| Fault::refused("expected nonempty text"))
    }

    pub fn hex(value: &Value, size: usize) -> Result<&str> {
        let text = Self::text(value)?;
        Self::require(
            text.len() == size
                && text
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
            "invalid hexadecimal identity",
        )?;
        Ok(text)
    }

    pub fn number(value: &Value) -> Result<u64> {
        value
            .as_u64()
            .ok_or_else(|| Fault::refused("expected an unsigned integer"))
    }

    pub fn decimal(value: &Value) -> Result<&str> {
        let text = Self::text(value)?;
        Self::require(
            !text.starts_with('0')
                && text.bytes().all(|byte| byte.is_ascii_digit())
                && text.parse::<u64>().is_ok(),
            "invalid native attempt identity",
        )?;
        Ok(text)
    }

    pub fn observer(value: &Value) -> Result<()> {
        Self::fields(value, &["run", "attempt", "wharf"])?;
        Self::decimal(&value["run"])?;
        Self::decimal(&value["attempt"])?;
        Self::hex(&value["wharf"], 40)?;
        Ok(())
    }

    pub fn names(value: &Value) -> Result<Vec<String>> {
        let list = value
            .as_array()
            .ok_or_else(|| Fault::refused("expected a name list"))?;
        let names = list
            .iter()
            .map(|value| {
                let name = Self::text(value)?;
                Self::require(
                    name.bytes().all(|byte| {
                        byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"_-".contains(&byte)
                    }),
                    "invalid name",
                )?;
                Ok(name.to_string())
            })
            .collect::<Result<Vec<_>>>()?;
        Self::require(
            !names.is_empty() && names.len() == names.iter().collect::<BTreeSet<_>>().len(),
            "empty or duplicate name list",
        )?;
        Ok(names)
    }

    pub fn authority(value: &str) -> Result<()> {
        let host = value
            .strip_prefix("https://")
            .ok_or_else(|| Fault::refused("authority requires public HTTPS"))?;
        Self::require(
            !host.is_empty()
                && host.bytes().all(|byte| {
                    byte.is_ascii_lowercase() || byte.is_ascii_digit() || b".-".contains(&byte)
                }),
            "authority must be a normalized HTTPS origin",
        )
    }

    pub fn reference(value: &Value, authority: &str) -> Result<()> {
        let fields = value
            .as_object()
            .ok_or_else(|| Fault::refused("reference is not an object"))?;
        let full = fields.contains_key("mime") || fields.contains_key("name");
        Self::fields(
            value,
            if full {
                &["url", "sha256", "size", "mime", "name"]
            } else {
                &["url", "sha256", "size"]
            },
        )?;
        Self::authority(authority)?;
        let url = Self::text(&value["url"])?;
        let path = url
            .strip_prefix(&format!("{authority}/"))
            .ok_or_else(|| Fault::refused("reference has a foreign authority"))?;
        Self::require(
            !path.contains(['?', '#', '%', '\\', '@'])
                && path.split('/').all(|part| !matches!(part, "" | "." | "..")),
            "reference is not an anchored public path",
        )?;
        Self::hex(&value["sha256"], 64)?;
        Self::require(Self::number(&value["size"])? > 0, "empty reference")?;
        if full {
            Self::text(&value["mime"])?;
            let name = Self::text(&value["name"])?;
            Self::require(
                !name.contains('/') && path.ends_with(&format!("/{name}")),
                "reference filename differs",
            )?;
        }
        Ok(())
    }

    pub fn read<R: Read>(reader: &R, reference: &Value, authority: &str) -> Result<Vec<u8>> {
        Self::reference(reference, authority)?;
        let size = Self::number(&reference["size"])?;
        Self::require(size <= ARCHIVE as u64, "public reference exceeds bounds")?;
        let raw = reader.bytes(Self::text(&reference["url"])?, size as usize)?;
        Self::require(
            raw.len() as u64 == size && Self::sha(&raw) == Self::text(&reference["sha256"])?,
            "public bytes differ from their size or digest",
        )?;
        Ok(raw)
    }

    pub fn document<R: Read>(reader: &R, url: &str, canonical: bool) -> Result<Value> {
        Self::parse(&reader.bytes(url, LIMIT)?, canonical)
    }
}
