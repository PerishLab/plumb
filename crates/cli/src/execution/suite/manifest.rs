use super::model::{Package, Target};
use std::path::Path;
use toml::Value;

pub(super) struct Manifest {
    value: Value,
}

impl Manifest {
    pub fn read(package: &Package) -> Result<Self, String> {
        let text = std::fs::read_to_string(&package.manifest)
            .map_err(|error| format!("cannot read {}: {error}", package.manifest.display()))?;
        let value: Value =
            toml::from_str(&text).map_err(|error| format!("invalid package manifest: {error}"))?;
        let name = value.get("package").and_then(|value| value.get("name"));
        if name.and_then(Value::as_str) != Some(package.name.as_str()) {
            return Err("Cargo package identity differs from its manifest".into());
        }
        if value.get("cargo-features").is_some() {
            return Err("unstable manifest features need original Cargo execution".into());
        }
        Ok(Self { value })
    }

    pub fn custom(&self, package: &Package, target: &Target) -> Result<bool, String> {
        let role = target.role()?;
        let directory = package
            .manifest
            .parent()
            .ok_or("package has no directory")?;
        let source = target
            .source
            .strip_prefix(directory)
            .map_err(|_| "target source is outside its package directory")?;
        let entries = self.entries(role)?;
        for entry in &entries {
            validate(entry)?;
        }
        let declared = entries.iter().any(|entry| {
            role == "lib" || entry.get("name").and_then(Value::as_str) == Some(&target.name)
        });
        let matched = entries
            .into_iter()
            .filter(|entry| matches(entry, target, source, role))
            .collect::<Vec<_>>();
        match matched.as_slice() {
            [entry] => setting(entry, "harness"),
            [] if !declared && implicit(package, target, source, role) => Ok(false),
            [] => Err(format!("cannot establish harness for {}", target.name)),
            _ => Err(format!("ambiguous target declarations for {}", target.name)),
        }
    }

    fn entries(&self, role: &str) -> Result<Vec<&Value>, String> {
        let Some(value) = self.value.get(role) else {
            return Ok(Vec::new());
        };
        if role == "lib" {
            return value
                .as_table()
                .map(|_| vec![value])
                .ok_or_else(|| "library declaration is not a table".into());
        }
        value
            .as_array()
            .map(|entries| entries.iter().collect())
            .ok_or_else(|| "target declarations are not an array".into())
    }
}

fn validate(entry: &Value) -> Result<(), String> {
    if !entry.is_table() {
        return Err("target declaration is not a table".into());
    }
    for key in ["name", "path"] {
        if entry.get(key).is_some_and(|value| !value.is_str()) {
            return Err(format!("target {key} is not text"));
        }
    }
    if let Some(path) = entry.get("path").and_then(Value::as_str) {
        let ordinary = Path::new(path)
            .components()
            .all(|part| matches!(part, std::path::Component::Normal(_)));
        if !ordinary {
            return Err("nonlocal target paths need original Cargo execution".into());
        }
    }
    setting(entry, "harness").map(|_| ())
}

fn matches(entry: &Value, target: &Target, source: &Path, role: &str) -> bool {
    let name = entry.get("name").and_then(Value::as_str);
    let path = entry.get("path").and_then(Value::as_str);
    if name.is_some_and(|name| name != target.name) {
        return false;
    }
    if path.is_some_and(|path| Path::new(path) != source) {
        return false;
    }
    role == "lib" || name.is_some() || path.is_some()
}

fn setting(entry: &Value, key: &str) -> Result<bool, String> {
    match entry.get(key) {
        None => Ok(false),
        Some(Value::Boolean(value)) => Ok(!value),
        Some(_) => Err(format!("target {key} is not a boolean")),
    }
}

fn implicit(package: &Package, target: &Target, source: &Path, role: &str) -> bool {
    match role {
        "lib" => source == Path::new("src/lib.rs") && target.name == package.name.replace('-', "_"),
        "bin" if source == Path::new("src/main.rs") => target.name == package.name,
        "bin" => discovered(source, "src/bin", &target.name),
        "test" => discovered(source, "tests", &target.name),
        "example" => discovered(source, "examples", &target.name),
        _ => false,
    }
}

fn discovered(source: &Path, parent: &str, name: &str) -> bool {
    source == Path::new(parent).join(format!("{name}.rs"))
        || source == Path::new(parent).join(name).join("main.rs")
}
