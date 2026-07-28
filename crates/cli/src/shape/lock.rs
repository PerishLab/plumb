use plumb::skill::stamp;
use std::path::{Path, PathBuf};

pub struct Lock {
    pub name: String,
    pub paths: Vec<String>,
    pub version: String,
    pub hash: String,
}

pub fn read(root: &Path) -> Vec<Lock> {
    let text = std::fs::read_to_string(root.join("plumb.toml")).unwrap_or_default();
    let Ok(doc) = text.parse::<toml::Table>() else {
        return Vec::new();
    };
    let Some(list) = doc.get("lock").and_then(toml::Value::as_array) else {
        return Vec::new();
    };
    list.iter().filter_map(one).collect()
}

pub fn held(root: &Path) -> Option<String> {
    let cargo = std::fs::read_to_string(root.join("Cargo.toml")).unwrap_or_default();
    if let Some(found) = mark(&cargo) {
        return Some(found);
    }
    for name in ["deno.json", "package.json"] {
        let text = std::fs::read_to_string(root.join(name)).unwrap_or_default();
        if let Some(found) = quoted(&text) {
            return Some(found);
        }
    }
    None
}

pub fn seal(root: &Path, lock: &Lock) -> Result<String, String> {
    let mut found = Vec::new();
    for spot in &lock.paths {
        let seat = root.join(spot);
        if !seat.exists() {
            return Err(format!("names {spot}, which does not exist"));
        }
        walk(&seat, &mut found)?;
    }
    found.sort();
    let mut bytes = Vec::new();
    for path in &found {
        let rel = path.strip_prefix(root).unwrap_or(path);
        bytes.extend_from_slice(rel.to_string_lossy().replace('\\', "/").as_bytes());
        bytes.push(0);
        let body = flat(std::fs::read(path).map_err(|error| error.to_string())?);
        bytes.extend_from_slice(body.len().to_string().as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(&body);
    }
    Ok(stamp(&bytes))
}

fn flat(body: Vec<u8>) -> Vec<u8> {
    let mut out = Vec::with_capacity(body.len());
    let mut held = body.iter().peekable();
    while let Some(byte) = held.next() {
        if *byte == b'\r' && held.peek() == Some(&&b'\n') {
            continue;
        }
        out.push(*byte);
    }
    out
}

fn walk(seat: &Path, found: &mut Vec<PathBuf>) -> Result<(), String> {
    if seat.is_file() {
        found.push(seat.to_path_buf());
        return Ok(());
    }
    let listed = std::fs::read_dir(seat).map_err(|error| error.to_string())?;
    for entry in listed {
        let entry = entry.map_err(|error| error.to_string())?;
        walk(&entry.path(), found)?;
    }
    Ok(())
}

fn one(value: &toml::Value) -> Option<Lock> {
    let name = text(value, "name")?;
    let paths = value
        .get("paths")
        .and_then(toml::Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(toml::Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    Some(Lock {
        name,
        paths,
        version: text(value, "version").unwrap_or_default(),
        hash: text(value, "hash").unwrap_or_default(),
    })
}

fn text(value: &toml::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(toml::Value::as_str)
        .map(str::to_string)
}

fn mark(text: &str) -> Option<String> {
    text.lines()
        .find(|line| line.trim_start().starts_with("version = \""))
        .and_then(|line| line.split('"').nth(1))
        .map(str::to_string)
}

fn quoted(text: &str) -> Option<String> {
    let seat = text.find("\"version\"")?;
    text[seat..].split('"').nth(3).map(str::to_string)
}

pub type Found = Vec<(&'static str, String)>;

pub fn locked(held: &super::Shape) -> Found {
    let mut found = Vec::new();
    for lock in &held.locks {
        let seen = match seal(&held.root, lock) {
            Ok(seen) => seen,
            Err(why) => {
                found.push(("out of true", format!("lock {} {why}", lock.name)));
                continue;
            }
        };
        if seen != lock.hash {
            found.push(("out of true", stale(lock)));
            continue;
        }
        if held.version.as_deref().unwrap_or_default() != lock.version {
            found.push(("out of true", moved(lock, held.version.as_deref())));
        }
    }
    found
}

fn stale(lock: &Lock) -> String {
    format!(
        "lock {} covers {} and they have changed since the affirmation at {}; re-read them and run plumb lock",
        lock.name,
        lock.paths.join(" "),
        lock.version
    )
}

fn moved(lock: &Lock, held: Option<&str>) -> String {
    format!(
        "lock {} was affirmed at {}, the repository stands at {}; re-read {} and run plumb lock",
        lock.name,
        lock.version,
        held.unwrap_or("an unread version"),
        lock.paths.join(" ")
    )
}
