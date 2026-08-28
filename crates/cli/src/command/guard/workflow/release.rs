use std::path::Path;

const HELD: &str = "plumb-release-identity";

pub(super) fn fingerprint(
    guard: bool,
    source: (&Path, Option<&str>, bool),
    path: &str,
    meta: &str,
) -> Result<Option<String>, String> {
    if !guard || source.2 {
        return Ok(Some(meta.to_string()));
    }
    if path.starts_with(".plumb/releases/") && path.ends_with("/datum.toml") {
        return Ok(None);
    }
    let normalized = match path {
        "Cargo.lock" => cargo(&object(source.0, source.1, path)?, true)?,
        path if path.ends_with("Cargo.toml") => cargo(&object(source.0, source.1, path)?, false)?,
        path if path.ends_with("package.json") => package(&object(source.0, source.1, path)?)?,
        path if path.ends_with("Chart.yaml") => chart(&object(source.0, source.1, path)?)?,
        _ => return Ok(Some(meta.to_string())),
    };
    let mode = meta.split_once(' ').map_or("", |(mode, _)| mode);
    Ok(Some(format!("{mode} {normalized}")))
}

fn object(root: &Path, revision: Option<&str>, path: &str) -> Result<Vec<u8>, String> {
    let object = revision.map_or_else(|| format!(":{path}"), |held| format!("{held}:{path}"));
    let output = plumb::config::detached("git")
        .arg("-C")
        .arg(root)
        .args(["show", &object])
        .output()
        .map_err(|error| format!("cannot read release identity leaf {path}: {error}"))?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(format!("cannot read release identity leaf {path}"))
    }
}

fn cargo(bytes: &[u8], lock: bool) -> Result<String, String> {
    let table: toml::Table = std::str::from_utf8(bytes)
        .map_err(|error| format!("release Cargo identity is not UTF-8: {error}"))?
        .parse()
        .map_err(|error| format!("cannot parse release Cargo identity: {error}"))?;
    let mut doc = toml::Value::Table(table);
    if lock {
        let packages = doc
            .get_mut("package")
            .and_then(toml::Value::as_array_mut)
            .ok_or_else(|| "release Cargo lock has no package array".to_string())?;
        for package in packages {
            let Some(table) = package.as_table_mut() else {
                continue;
            };
            if !table.contains_key("source") && table.contains_key("version") {
                table.insert("version".into(), toml::Value::String(HELD.into()));
            }
        }
    } else {
        manifest(&mut doc);
    }
    serde_json::to_string(&doc).map_err(|error| error.to_string())
}

fn manifest(value: &mut toml::Value) {
    let Some(table) = value.as_table_mut() else {
        return;
    };
    if let Some(package) = table.get_mut("package").and_then(toml::Value::as_table_mut)
        && package.contains_key("version")
    {
        package.insert("version".into(), toml::Value::String(HELD.into()));
    }
    if let Some(package) = table
        .get_mut("workspace")
        .and_then(toml::Value::as_table_mut)
        .and_then(|workspace| workspace.get_mut("package"))
        .and_then(toml::Value::as_table_mut)
        && package.contains_key("version")
    {
        package.insert("version".into(), toml::Value::String(HELD.into()));
    }
    if table.contains_key("path") && table.contains_key("version") {
        table.insert("version".into(), toml::Value::String(HELD.into()));
    }
    for (_, child) in table.iter_mut() {
        manifest(child);
    }
}

fn package(bytes: &[u8]) -> Result<String, String> {
    let mut doc: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|error| format!("cannot parse release package identity: {error}"))?;
    if let Some(version) = doc.get_mut("version") {
        *version = serde_json::Value::String(HELD.into());
    }
    serde_json::to_string(&doc).map_err(|error| error.to_string())
}

fn chart(bytes: &[u8]) -> Result<String, String> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| format!("release chart identity is not UTF-8: {error}"))?;
    Ok(text
        .lines()
        .map(|line| {
            if line.starts_with("version:") || line.starts_with("appVersion:") {
                line.split_once(':')
                    .map_or_else(|| line.to_string(), |(name, _)| format!("{name}: {HELD}"))
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n"))
}
