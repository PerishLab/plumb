use super::model::App;
use std::path::Path;

pub struct Marks {
    pub commit: String,
    pub version: String,
}

pub fn marks(app: &App) -> Result<Marks, String> {
    let commit = super::process::text("git", &["rev-parse", "--short", "HEAD"], &app.root)?;
    let version = version(&app.root)?;
    Ok(Marks { commit, version })
}

fn version(root: &Path) -> Result<String, String> {
    let path = root.join("Cargo.toml");
    if !path.exists() {
        return Ok(String::new());
    }
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let doc = text
        .parse::<toml::Table>()
        .map_err(|error| format!("cannot parse {}: {error}", path.display()))?;
    let version = doc
        .get("workspace")
        .and_then(|value| value.get("package"))
        .and_then(|value| value.get("version"))
        .and_then(toml::Value::as_str)
        .or_else(|| {
            doc.get("package")
                .and_then(|value| value.get("version"))
                .and_then(toml::Value::as_str)
        })
        .unwrap_or("")
        .to_string();
    Ok(version)
}

pub fn stamp(app: &App) -> Result<String, String> {
    let html = std::fs::read_to_string(app.index())
        .map_err(|error| format!("cannot read {}: {error}", app.index().display()))?;
    let start = html
        .find("/assets/")
        .ok_or_else(|| format!("no fingerprinted asset in {}", app.index().display()))?;
    let tail = &html[start..];
    let end = tail
        .find(".js")
        .map(|at| at + 3)
        .ok_or_else(|| format!("no fingerprinted asset in {}", app.index().display()))?;
    let stamp = &tail[..end];
    let name = stamp.rsplit('/').next().unwrap_or("");
    if !name.contains('-') {
        return Err(format!("asset is not fingerprinted: {stamp}"));
    }
    Ok(stamp.to_string())
}

pub fn routes(app: &App) -> Result<Vec<String>, String> {
    let Ok(text) = std::fs::read_to_string(app.atlas()) else {
        return Ok(Vec::new());
    };
    let mut found = Vec::new();
    let mut rest = text.as_str();
    while let Some((_, tail)) = rest.split_once("<loc>") {
        let Some((url, after)) = tail.split_once("</loc>") else {
            break;
        };
        if let Some(path) = route(url) {
            found.push(path);
        }
        rest = after;
    }
    Ok(found)
}

pub fn deep(routes: &[String]) -> Option<&str> {
    routes
        .iter()
        .map(String::as_str)
        .find(|route| *route != "/")
}

fn route(url: &str) -> Option<String> {
    let tail = url.split_once("://").map(|(_, tail)| tail).unwrap_or(url);
    let at = tail.find('/').unwrap_or(tail.len());
    let path = &tail[at..];
    let path = if path.is_empty() { "/" } else { path };
    Some(if path.len() > 1 {
        path.trim_end_matches('/').to_string()
    } else {
        path.to_string()
    })
}
