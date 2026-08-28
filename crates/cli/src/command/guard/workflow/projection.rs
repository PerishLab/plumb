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
        Some("yaml" | "yml") => yaml(path, bytes),
        _ => Err(format!(
            "cannot project {path}: projected leaves must be JSON, TOML, or YAML"
        )),
    }
}

fn yaml(path: &str, bytes: &[u8]) -> Result<serde_json::Value, String> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| format!("cannot project {path} as YAML: {error}"))?;
    let mut document = serde_json::Map::new();
    let mut layout = Vec::new();
    for segment in text.split_inclusive('\n') {
        let (line, ending) = segment
            .strip_suffix('\n')
            .map_or((segment, false), |line| (line, true));
        let field = (!line.starts_with(char::is_whitespace) && !line.starts_with('#'))
            .then(|| line.split_once(':'))
            .flatten()
            .filter(|(key, _)| {
                !key.is_empty()
                    && key
                        .chars()
                        .all(|held| held.is_ascii_alphanumeric() || matches!(held, '_' | '-'))
            });
        if let Some((key, value)) = field {
            if document
                .insert(
                    key.to_string(),
                    serde_json::Value::String(value.to_string()),
                )
                .is_some()
            {
                return Err(format!(
                    "cannot project {path} as YAML: duplicate key {key}"
                ));
            }
            layout.push(serde_json::json!({ "key": key, "ending": ending }));
        } else {
            layout.push(serde_json::Value::String(segment.to_string()));
        }
    }
    document.insert("$plumb".into(), serde_json::Value::Array(layout));
    Ok(serde_json::Value::Object(document))
}
