use std::path::Path;

pub(super) fn read(snapshot: &plumb::snapshot::Snapshot) -> Vec<String> {
    let rules =
        crate::catalog::set::read("structure").expect("source rules follow catalogue preparation");
    let source = &rules["source"];
    snapshot
        .entries()
        .iter()
        .map(|entry| entry.path())
        .filter(|path| outside(path, source))
        .map(str::to_string)
        .collect()
}

fn outside(path: &str, rules: &toml::Value) -> bool {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() < 3 || parts[0] != "packages" {
        return false;
    }
    if parts.contains(&"node_modules")
        || matches!(
            parts[2],
            "src" | "tests" | "dist" | "target" | ".svelte-kit"
        )
    {
        return false;
    }
    let file = Path::new(path);
    let Some(extension) = file.extension().and_then(|value| value.to_str()) else {
        return false;
    };
    if !named(rules, "extensions", extension) {
        return false;
    }
    let name = file
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    !(parts.len() == 3 && named(rules, "configurations", name))
}

fn named(rules: &toml::Value, key: &str, name: &str) -> bool {
    rules[key]
        .as_array()
        .expect("source rule lists")
        .iter()
        .any(|entry| entry.as_str() == Some(name))
}
