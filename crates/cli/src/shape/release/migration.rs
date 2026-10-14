use std::path::Path;

pub fn migrate(root: &Path) -> Result<bool, String> {
    let path = root.join("plumb.toml");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(false);
    };
    let Some(migrated) = rewrite(&text)? else {
        return Ok(false);
    };
    std::fs::write(&path, migrated)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    Ok(true)
}

pub fn rewrite(text: &str) -> Result<Option<String>, String> {
    let held: toml::Table = text
        .parse()
        .map_err(|error| format!("cannot parse plumb.toml: {error}"))?;
    let Some(product) = held
        .get("release")
        .and_then(|release| release.get("product"))
    else {
        return Ok(None);
    };
    let product = product
        .as_str()
        .ok_or("plumb.toml [release] product is not a string")?;
    super::token("[release] product", product, false)?;
    let ns = held
        .get("identity")
        .and_then(|identity| identity.get("ns"))
        .and_then(toml::Value::as_str);
    if ns.is_some_and(|ns| ns != product) {
        return Err(format!(
            "plumb.toml declares [release] product {product} beside [identity] ns {}; remove one by hand",
            ns.unwrap_or_default()
        ));
    }
    let mut section = String::new();
    let mut removed = 0;
    let mut migrated = String::new();
    for line in text.split_inclusive('\n') {
        let held = line.split('#').next().unwrap_or_default().trim();
        if held.starts_with('[') {
            section = held.to_string();
            if section == "[release]" && ns.is_none() {
                migrated.push_str(&format!("[identity]\nns = \"{product}\"\n\n"));
            }
        }
        let key = held.strip_prefix("product").map(str::trim_start);
        if section == "[release]" && key.is_some_and(|rest| rest.starts_with('=')) {
            removed += 1;
            continue;
        }
        migrated.push_str(line);
    }
    let check: toml::Table = migrated
        .parse()
        .map_err(|error| format!("cannot migrate plumb.toml mechanically: {error}"))?;
    let named = check
        .get("identity")
        .and_then(|identity| identity.get("ns"))
        .and_then(toml::Value::as_str);
    let left = check
        .get("release")
        .and_then(|release| release.get("product"));
    if removed != 1 || named != Some(product) || left.is_some() {
        return Err(
            "cannot migrate plumb.toml mechanically; move [release] product to [identity] ns by hand"
                .into(),
        );
    }
    Ok(Some(migrated))
}
