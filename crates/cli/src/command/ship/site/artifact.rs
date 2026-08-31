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
