use std::collections::BTreeSet;
use std::path::Path;

pub struct Shape {
    pub wrappers: BTreeSet<String>,
    pub dirs: BTreeSet<String>,
    pub block: Option<i64>,
    pub path: Option<i64>,
    pub grants: BTreeSet<String>,
    pub laws: bool,
    pub unread: Option<String>,
    pub lanes: BTreeSet<String>,
    pub ships: BTreeSet<String>,
    pub ignore: String,
    pub listed: BTreeSet<String>,
    pub bounds: Vec<String>,
    pub root: std::path::PathBuf,
    pub inits: bool,
    pub rust: bool,
}

fn names(root: &Path, under: &str, suffix: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let Ok(entries) = std::fs::read_dir(root.join(under)) else {
        return found;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(stem) = name.strip_suffix(suffix) {
            found.insert(stem.to_string());
        }
    }
    found
}

fn dirs(root: &Path) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return found;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let skip = name.starts_with('.') || name == "target" || name == "node_modules";
        if entry.path().is_dir() && !skip {
            found.insert(name);
        }
    }
    found
}

fn ships(root: &Path) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    if root.join("manage.sh").exists() {
        found.insert("binary".to_string());
    }
    if root.join("deno.json").exists() {
        found.insert("jsr".to_string());
    }
    for seat in ["packages"] {
        let Ok(entries) = std::fs::read_dir(root.join(seat)) else {
            continue;
        };
        for entry in entries.flatten() {
            if entry.path().join("deno.json").exists() {
                found.insert("jsr".to_string());
            }
            if minted(&entry.path().join("package.json")) {
                found.insert("npm".to_string());
            }
        }
    }
    found
}

fn minted(path: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    text.contains("\"publishConfig\"") || text.contains("\"files\"")
}

fn listed(root: &Path) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for seat in [".runseal/wrappers/init.ts", ".runseal/lib/init/init.ts"] {
        let Ok(text) = std::fs::read_to_string(root.join(seat)) else {
            continue;
        };
        for line in text.lines() {
            let Some(at) = line.find(".runseal/wrappers/") else {
                continue;
            };
            let rest = &line[at + 18..];
            if let Some(end) = rest.find(".ts") {
                found.insert(rest[..end].to_string());
            }
        }
    }
    found
}

fn bounds(doc: Option<&toml::Value>) -> Vec<String> {
    let mut found = Vec::new();
    let Some(list) = doc
        .and_then(|value| value.get("boundary"))
        .and_then(toml::Value::as_array)
    else {
        return found;
    };
    for edge in list {
        let Some(paths) = edge.get("paths").and_then(toml::Value::as_array) else {
            continue;
        };
        for path in paths.iter().filter_map(toml::Value::as_str) {
            if !path.contains('*') {
                found.push(path.to_string());
            }
        }
    }
    found
}

pub fn read(root: &Path) -> Shape {
    let laws = root.join("negentropy.toml");
    let text = std::fs::read_to_string(&laws).unwrap_or_default();
    let read = text.parse::<toml::Table>();
    let unread = read.as_ref().err().map(|error| error.to_string());
    let doc = read.ok().map(toml::Value::Table);
    let limit = |key: &str| {
        doc.as_ref()
            .and_then(|value| value.get("limit"))
            .and_then(|value| value.get(key))
            .and_then(toml::Value::as_integer)
    };
    let mut grants = BTreeSet::new();
    if let Some(list) = doc
        .as_ref()
        .and_then(|value| value.get("grant"))
        .and_then(toml::Value::as_array)
    {
        for entry in list {
            if let Some(name) = entry.get("syntax").and_then(toml::Value::as_str) {
                grants.insert(name.to_string());
            }
        }
    }
    Shape {
        wrappers: names(root, ".runseal/wrappers", ".ts"),
        dirs: dirs(root),
        block: limit("block"),
        path: limit("path"),
        grants,
        laws: laws.exists(),
        unread,
        lanes: names(root, ".forgejo/workflows", ".yml"),
        ships: ships(root),
        ignore: std::fs::read_to_string(root.join(".gitignore")).unwrap_or_default(),
        rust: root.join("Cargo.toml").exists(),
        listed: listed(root),
        bounds: bounds(doc.as_ref()),
        root: root.to_path_buf(),
        inits: root.join(".runseal/wrappers/init.ts").exists()
            || root.join(".runseal/lib/init/init.ts").exists(),
    }
}
