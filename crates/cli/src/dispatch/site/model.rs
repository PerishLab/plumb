use serde_json::Value;
use std::path::{Component, Path, PathBuf};

pub struct App {
    pub root: PathBuf,
    pub seat: PathBuf,
    pub dist: PathBuf,
    pub package: String,
    pub worker: String,
}

impl App {
    pub fn read(root: &Path) -> Result<Self, String> {
        let seats = seats(root)?;
        let [seat] = seats.as_slice() else {
            return Err(format!(
                "expected exactly one apps/*/wrangler.jsonc, found {}",
                seats.len()
            ));
        };
        let config = std::fs::read_to_string(seat.join("wrangler.jsonc"))
            .map_err(|error| format!("cannot read site config: {error}"))?;
        let worker =
            field(&config, "name").ok_or_else(|| "site config has no worker name".to_string())?;
        let directory = field(&config, "directory")
            .ok_or_else(|| "site config has no assets directory".to_string())?;
        let relative = safe(&directory)?;
        let package = package(seat)?;
        Ok(Self {
            root: root.to_path_buf(),
            seat: seat.to_path_buf(),
            dist: seat.join(relative),
            package,
            worker,
        })
    }

    pub fn index(&self) -> PathBuf {
        self.dist.join("index.html")
    }

    pub fn atlas(&self) -> PathBuf {
        self.dist.join("sitemap.xml")
    }
}

fn seats(root: &Path) -> Result<Vec<PathBuf>, String> {
    let apps = root.join("apps");
    let entries = std::fs::read_dir(&apps)
        .map_err(|error| format!("cannot read {}: {error}", apps.display()))?;
    let mut found = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.join("wrangler.jsonc").is_file())
        .collect::<Vec<_>>();
    found.sort();
    Ok(found)
}

fn package(seat: &Path) -> Result<String, String> {
    let path = seat.join("package.json");
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let doc: Value = serde_json::from_str(&text)
        .map_err(|error| format!("cannot parse {}: {error}", path.display()))?;
    doc.get("name")
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("{} has no package name", path.display()))
}

fn safe(value: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(value);
    let held = path
        .components()
        .all(|part| matches!(part, Component::Normal(_) | Component::CurDir));
    if held {
        Ok(path)
    } else {
        Err(format!("site assets directory is not relative: {value}"))
    }
}

fn field(text: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let tail = text.split_once(&needle)?.1;
    let tail = tail.split_once(':')?.1.trim_start();
    let tail = tail.strip_prefix('"')?;
    let end = tail.find('"')?;
    Some(tail[..end].to_string())
}
