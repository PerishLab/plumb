use serde_json::Value;
use std::collections::BTreeMap;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Worker {
    pub name: String,
    #[serde(rename = "account_id")]
    pub account: String,
    #[serde(rename = "compatibility_date")]
    date: String,
    #[serde(rename = "workers_dev")]
    development: bool,
    pub assets: Assets,
    previews: BTreeMap<String, Value>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Assets {
    pub directory: String,
    #[serde(rename = "not_found_handling")]
    handling: String,
}

impl Worker {
    pub fn read(value: serde_json::Value) -> Result<Self, String> {
        let worker: Self = serde_json::from_value(value).map_err(|error| error.to_string())?;
        worker.judge()?;
        Ok(worker)
    }

    fn judge(&self) -> Result<(), String> {
        if self.development || !self.previews.is_empty() {
            return Err("static Worker must disable workers_dev and declare empty previews".into());
        }
        if !account(&self.account) {
            return Err("static Worker needs a lowercase 32-digit hex account".into());
        }
        if !slug(&self.name) {
            return Err("static Worker needs a lowercase slug of at most 63 bytes".into());
        }
        if !super::calendar::valid(&self.date) {
            return Err("static Worker needs an explicit real calendar date".into());
        }
        if self.assets.handling != "404-page" {
            return Err("static assets must use 404-page handling".into());
        }
        self.assets.path()?;
        Ok(())
    }
}

impl Assets {
    pub fn path(&self) -> Result<&str, String> {
        let value = self.directory.strip_prefix("./").unwrap_or(&self.directory);
        let platform = value.contains('\\') || value.as_bytes().get(1) == Some(&b':');
        let segments = value
            .split('/')
            .any(|part| part.is_empty() || part.starts_with('.'));
        if platform || segments {
            return Err(
                "static assets need a normalized relative non-hidden build directory".into(),
            );
        }
        Ok(value)
    }
}

pub(super) fn account(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

pub(super) fn slug(value: &str) -> bool {
    let edge = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit();
    if value.is_empty() || value.len() > 63 {
        return false;
    }
    let bytes = value.as_bytes();
    edge(bytes[0])
        && edge(bytes[bytes.len() - 1])
        && bytes.iter().all(|byte| edge(*byte) || *byte == b'-')
}
