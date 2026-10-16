use super::process;
use std::path::Path;

#[derive(Default)]
pub struct Policy {
    pub flags: Vec<String>,
    minimum: Option<u64>,
}

pub fn exclusions(root: &Path) -> Result<Vec<String>, String> {
    Ok(Policy::read(root)?.flags)
}

impl Policy {
    pub fn read(root: &Path) -> Result<Self, String> {
        let bytes = read(root, "minimumReleaseAgeExclude")?;
        let mut patterns = patterns(&bytes)?;
        if !patterns.iter().any(|pattern| pattern == "@perishlab/*") {
            patterns.push("@perishlab/*".into());
        }
        let bytes = read(root, "minimumReleaseAge")?;
        let minimum = if bytes.iter().all(u8::is_ascii_whitespace) {
            None
        } else {
            serde_json::from_slice::<Option<u64>>(&bytes)
                .map_err(|_| "unsupported effective npm minimum release age")?
        };
        use sha2::Digest;
        let encoded = serde_json::to_vec(&patterns).map_err(|_| "cannot encode npm age policy")?;
        let digest = sha2::Sha256::digest(encoded);
        let label = minimum.map_or_else(
            || "provider default".into(),
            |value| format!("{value} minutes"),
        );
        eprintln!(
            "npm release-age policy: minimum={label}; exclusions={} sha256={digest:x}; first-party=@perishlab/*",
            patterns.len()
        );
        let flags = patterns
            .into_iter()
            .map(|pattern| format!("--config.minimum-release-age-exclude={pattern}"))
            .collect();
        Ok(Self { flags, minimum })
    }

    pub fn environment(&self) -> Result<crate::config::Environment, String> {
        let bind = self
            .minimum
            .map(|value| {
                (
                    "pnpm_config_minimum_release_age".to_string(),
                    crate::config::Binding::Value(value.to_string()),
                )
            })
            .into_iter()
            .collect();
        crate::config::Contract {
            inherit: Vec::new(),
            managed: Vec::new(),
            reject: Vec::new(),
            bind,
        }
        .capture([])
    }
}

pub fn cache(home: &Path) -> Result<String, String> {
    let path = home.join("cache").join("guard").join("pnpm");
    let path = path.to_str().ok_or("Guard pnpm cache path is not text")?;
    Ok(format!("--config.cache-dir={path}"))
}

fn read(root: &Path, setting: &str) -> Result<Vec<u8>, String> {
    let argv = [
        "pnpm",
        "--config.ignore-pnpmfile=true",
        "config",
        "get",
        setting,
        "--json",
    ]
    .map(str::to_string);
    process::run(root, "packages", &argv)
}

fn patterns(bytes: &[u8]) -> Result<Vec<String>, String> {
    if bytes.iter().all(u8::is_ascii_whitespace) {
        return Ok(Vec::new());
    }
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|_| "cannot read effective npm release-age exclusions")?;
    match value {
        serde_json::Value::Null => Ok(Vec::new()),
        serde_json::Value::String(value) => Ok(vec![value]),
        serde_json::Value::Array(values) => values
            .into_iter()
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_string)
                    .ok_or_else(|| "npm release-age exclusion is not text".into())
            })
            .collect(),
        _ => Err("unsupported effective npm release-age exclusions".into()),
    }
}

#[cfg(test)]
mod declared;
#[cfg(test)]
mod frozen;
#[cfg(test)]
mod tests;
