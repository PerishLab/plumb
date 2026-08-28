use std::path::Path;

pub fn document(path: &str, bytes: &[u8]) -> Result<serde_json::Value, String> {
    match Path::new(path).extension().and_then(|held| held.to_str()) {
        Some("json") => serde_json::from_slice(bytes)
            .map_err(|error| format!("cannot project {path} as JSON: {error}")),
        Some("toml") => {
            let text = std::str::from_utf8(bytes)
                .map_err(|error| format!("cannot project {path} as TOML: {error}"))?;
            let value: toml::Table = text
                .parse()
                .map_err(|error| format!("cannot project {path} as TOML: {error}"))?;
            serde_json::to_value(value)
                .map_err(|error| format!("cannot project {path} as TOML: {error}"))
        }
        _ => Err(format!(
            "cannot project {path}: projected leaves must be JSON or TOML"
        )),
    }
}
