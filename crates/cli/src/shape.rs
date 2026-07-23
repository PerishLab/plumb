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
    pub runseal: bool,
    pub guard_lane: Option<String>,
    pub edition: Option<String>,
    pub binary: bool,
    pub clap: bool,
    pub substrate: bool,
    pub deno: String,
    pub root_package: Option<String>,
    pub packages: Vec<(String, String)>,
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

fn json_field(text: &str, key: &str) -> Option<String> {
    let marker = format!("\"{key}\"");
    let at = text.find(&marker)?;
    let rest = &text[at + marker.len()..];
    let colon = rest.find(':')?;
    let tail = &rest[colon + 1..];
    let open = tail.find('"')?;
    let value = &tail[open + 1..];
    let close = value.find('"')?;
    Some(value[..close].to_string())
}

fn package_name(dir: &Path) -> Option<String> {
    for seat in ["deno.json", "package.json"] {
        if let Ok(text) = std::fs::read_to_string(dir.join(seat))
            && let Some(name) = json_field(&text, "name")
        {
            return Some(name);
        }
    }
    None
}

fn packages(root: &Path) -> Vec<(String, String)> {
    let mut held = Vec::new();
    let Ok(entries) = std::fs::read_dir(root.join("packages")) else {
        return held;
    };
    for entry in entries.flatten() {
        let dir = entry.file_name().to_string_lossy().to_string();
        if let Some(name) = package_name(&entry.path()) {
            held.push((dir, name));
        }
    }
    held
}

fn root_package(root: &Path) -> Option<String> {
    if let Ok(text) = std::fs::read_to_string(root.join("deno.json"))
        && text.contains("\"exports\"")
    {
        return json_field(&text, "name");
    }
    if minted(&root.join("package.json"))
        && let Ok(text) = std::fs::read_to_string(root.join("package.json"))
    {
        return json_field(&text, "name");
    }
    None
}

fn denos(root: &Path) -> String {
    std::fs::read_to_string(root.join(".runseal/deno.json")).unwrap_or_default()
}

fn manifests(root: &Path) -> Vec<String> {
    let mut held = Vec::new();
    for seat in ["Cargo.toml", "app/Cargo.toml"] {
        if let Ok(text) = std::fs::read_to_string(root.join(seat)) {
            held.push(text);
        }
    }
    if let Ok(entries) = std::fs::read_dir(root.join("crates")) {
        for entry in entries.flatten() {
            if let Ok(text) = std::fs::read_to_string(entry.path().join("Cargo.toml")) {
                held.push(text);
            }
        }
    }
    held
}

fn binary(root: &Path) -> bool {
    if manifests(root).iter().any(|text| text.contains("[[bin]]")) {
        return true;
    }
    let mut seats = vec![root.to_path_buf(), root.join("app")];
    if let Ok(entries) = std::fs::read_dir(root.join("crates")) {
        for entry in entries.flatten() {
            seats.push(entry.path());
        }
    }
    seats.iter().any(|seat| {
        seat.join("Cargo.toml").is_file()
            && (seat.join("src/main.rs").is_file() || seat.join("src/bin").is_dir())
    })
}

fn substrate(root: &Path) -> bool {
    std::fs::read_to_string(root.join("Cargo.lock"))
        .map(|lock| lock.contains("name = \"plumb\""))
        .unwrap_or(false)
}

fn edition(root: &Path) -> Option<String> {
    let text = std::fs::read_to_string(root.join("Cargo.toml")).ok()?;
    let doc = text.parse::<toml::Table>().ok()?;
    let held = doc
        .get("workspace")
        .and_then(|value| value.get("package"))
        .or_else(|| doc.get("package"));
    held.and_then(|value| value.get("edition"))
        .and_then(toml::Value::as_str)
        .map(str::to_string)
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
        runseal: root.join(".runseal").is_dir(),
        guard_lane: std::fs::read_to_string(root.join(".forgejo/workflows/guard.yml")).ok(),
        edition: edition(root),
        binary: binary(root),
        clap: manifests(root).iter().any(|text| {
            text.lines()
                .any(|line| line.trim_start().starts_with("clap"))
        }),
        substrate: substrate(root),
        deno: denos(root),
        root_package: root_package(root),
        packages: packages(root),
        listed: listed(root),
        bounds: bounds(doc.as_ref()),
        root: root.to_path_buf(),
        inits: root.join(".runseal/wrappers/init.ts").exists()
            || root.join(".runseal/lib/init/init.ts").exists(),
    }
}
