pub fn named(manifest: Option<&str>) -> Result<String, String> {
    let raw = manifest.ok_or_else(|| "staged tree requires plumb.toml".to_string())?;
    let doc: toml::Table = raw
        .parse()
        .map_err(|error| format!("cannot parse staged plumb.toml: {error}"))?;
    Ok(doc
        .get("release")
        .and_then(|held| held.get("product"))
        .and_then(toml::Value::as_str)
        .unwrap_or_default()
        .to_string())
}
