use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub struct Target {
    pub key: String,
    pub entry: PathBuf,
    pub base: PathBuf,
    pub label: String,
    pub context: &'static str,
    pub externs: BTreeMap<String, String>,
    pub local: bool,
}

pub struct Package<'a> {
    pub disk: &'a Path,
    pub shown: &'a str,
    pub local: bool,
}

const KINDS: [(&str, &str, &str); 3] = [
    ("test", "tests", "test"),
    ("example", "examples", "example"),
    ("bench", "benches", "bench"),
];

pub fn targets(package: &Package) -> Vec<Target> {
    let Some(manifest) = std::fs::read_to_string(package.disk.join("Cargo.toml"))
        .ok()
        .and_then(|text| text.parse::<toml::Table>().ok())
    else {
        return Vec::new();
    };
    let Some(name) = manifest
        .get("package")
        .and_then(|value| value.get("name"))
        .and_then(toml::Value::as_str)
    else {
        return Vec::new();
    };
    let lib = manifest.get("lib");
    let library = text(lib, "name").unwrap_or_else(|| name.replace('-', "_"));
    let mut externs = externs(&manifest);
    externs.insert(library.clone(), library.clone());
    let mut found = Vec::new();
    let mut add = |key: String, relative: String, context: &'static str| {
        let entry = package.disk.join(&relative);
        if entry.is_file() && !found.iter().any(|known: &Target| known.entry == entry) {
            found.push(Target {
                key,
                base: package.disk.to_path_buf(),
                label: package.shown.to_string(),
                entry,
                context,
                externs: externs.clone(),
                local: package.local,
            });
        }
    };
    let path = text(lib, "path").unwrap_or_else(|| "src/lib.rs".into());
    add(library, path, "production");
    if !package.local {
        return found;
    }
    for (relative, label) in listed(&manifest, "bin", "src/bin", package.disk) {
        add(format!("{label}[bin]"), relative, "production");
    }
    add(format!("{name}[bin]"), "src/main.rs".into(), "production");
    for (kind, directory, context) in KINDS {
        for (relative, label) in listed(&manifest, kind, directory, package.disk) {
            add(format!("{label}[{kind}]"), relative, context);
        }
    }
    let build = manifest
        .get("package")
        .and_then(|value| value.get("build"))
        .and_then(toml::Value::as_str)
        .unwrap_or("build.rs");
    add(format!("{name}[build]"), build.into(), "build");
    found
}

impl Target {
    pub fn shown(&self, disk: &Path) -> String {
        let relative = disk
            .strip_prefix(&self.base)
            .map(|path| path.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| disk.to_string_lossy().to_string());
        if self.label.is_empty() {
            relative
        } else {
            format!("{}/{relative}", self.label)
        }
    }
}

fn text(table: Option<&toml::Value>, key: &str) -> Option<String> {
    table?.get(key)?.as_str().map(str::to_string)
}

fn listed(
    manifest: &toml::Table,
    kind: &str,
    directory: &str,
    disk: &Path,
) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = Vec::new();
    let declared = manifest.get(kind).and_then(toml::Value::as_array);
    for entry in declared.into_iter().flatten() {
        let Some(name) = entry.get("name").and_then(toml::Value::as_str) else {
            continue;
        };
        let path = entry
            .get("path")
            .and_then(toml::Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| format!("{directory}/{name}.rs"));
        found.push((path, name.to_string()));
    }
    let Ok(entries) = std::fs::read_dir(disk.join(directory)) else {
        return found;
    };
    let mut discovered: Vec<(String, String)> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| automatic(directory, &entry.path()))
        .collect();
    discovered.sort();
    found.extend(discovered);
    found
}

fn automatic(directory: &str, path: &Path) -> Option<(String, String)> {
    let name = path.file_name()?.to_str()?.to_string();
    if path.is_dir() {
        return path
            .join("main.rs")
            .is_file()
            .then(|| (format!("{directory}/{name}/main.rs"), name));
    }
    let stem = name.strip_suffix(".rs")?;
    Some((format!("{directory}/{name}"), stem.to_string()))
}

fn externs(manifest: &toml::Table) -> BTreeMap<String, String> {
    let mut found = BTreeMap::new();
    let mut tables: Vec<&toml::Value> = ["dependencies", "dev-dependencies", "build-dependencies"]
        .iter()
        .filter_map(|key| manifest.get(*key))
        .collect();
    let targeted = manifest.get("target").and_then(toml::Value::as_table);
    for platform in targeted.into_iter().flat_map(|table| table.values()) {
        for key in ["dependencies", "dev-dependencies", "build-dependencies"] {
            tables.extend(platform.get(key));
        }
    }
    for table in tables.iter().filter_map(|value| value.as_table()) {
        for (alias, value) in table {
            let package = value
                .get("package")
                .and_then(toml::Value::as_str)
                .unwrap_or(alias);
            found.insert(alias.replace('-', "_"), package.replace('-', "_"));
        }
    }
    found
}
