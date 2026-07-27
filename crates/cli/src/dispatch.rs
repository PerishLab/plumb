use serde_json::Value as Json;
use std::path::{Path, PathBuf};

mod production;

type Found = Vec<(&'static str, String)>;

pub fn read(root: &Path) -> Option<Found> {
    let web = package(root)?;
    if !has_package(&web, "react") || !has_package(&web, "vite") || !executable_api(root) {
        return None;
    }

    let mut found = Found::new();
    let inherited = sidecar(root, &mut found);
    api(root, &mut found);
    web_plane(root, &web, inherited.as_deref(), &mut found);
    production::read(root, &mut found);
    Some(found)
}

fn package(root: &Path) -> Option<Json> {
    let text = std::fs::read_to_string(root.join("apps/web/package.json")).ok()?;
    serde_json::from_str(&text).ok()
}

fn has_package(doc: &Json, name: &str) -> bool {
    [
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ]
    .iter()
    .any(|seat| doc.get(seat).and_then(|map| map.get(name)).is_some())
}

fn executable_api(root: &Path) -> bool {
    let seat = root.join("crates/api");
    let Ok(text) = std::fs::read_to_string(seat.join("Cargo.toml")) else {
        return false;
    };
    let Ok(doc) = text.parse::<toml::Table>() else {
        return false;
    };
    let named = doc
        .get("package")
        .and_then(|package| package.get("name"))
        .and_then(toml::Value::as_str)
        == Some("api");
    named
        && (seat.join("src/main.rs").is_file()
            || seat.join("src/bin").is_dir()
            || doc.get("bin").and_then(toml::Value::as_array).is_some())
}

fn sidecar(root: &Path, found: &mut Found) -> Option<String> {
    let path = root.join("sidecar.toml");
    let Ok(text) = std::fs::read_to_string(&path) else {
        wrong(found, "web/api pair has no sidecar.toml");
        return None;
    };
    let doc = match text.parse::<toml::Table>() {
        Ok(doc) => toml::Value::Table(doc),
        Err(error) => {
            found.push((
                "blind",
                format!(
                    "cannot read sidecar.toml: {}",
                    error.to_string().lines().next().unwrap_or("")
                ),
            ));
            return None;
        }
    };

    let api = doc
        .get("sidecars")
        .and_then(toml::Value::as_array)
        .and_then(|list| {
            list.iter()
                .find(|entry| entry.get("name").and_then(toml::Value::as_str) == Some("api"))
        });
    match api {
        None => wrong(found, "sidecar does not declare the api role"),
        Some(api) => {
            if api.get("port").and_then(toml::Value::as_integer) != Some(0) {
                wrong(found, "sidecar api must lease port 0");
            }
            if api
                .get("ready")
                .and_then(|ready| ready.get("role"))
                .and_then(toml::Value::as_str)
                != Some("api")
            {
                wrong(found, "sidecar api must declare ready role api");
            }
            if !uses_port(api.get("health_url")) {
                wrong(found, "sidecar api health_url must use {port}");
            }
        }
    }

    let app = doc.get("app");
    if app
        .and_then(|app| app.get("name"))
        .and_then(toml::Value::as_str)
        != Some("web")
    {
        wrong(found, "sidecar does not declare the web app");
    }
    if app
        .and_then(|app| app.get("port"))
        .and_then(toml::Value::as_integer)
        != Some(0)
    {
        wrong(found, "sidecar web must lease port 0");
    }
    if !uses_port(app.and_then(|app| app.get("health_url"))) {
        wrong(found, "sidecar web health_url must use {port}");
    }

    let binding = app
        .and_then(|app| app.get("inherits_env"))
        .and_then(toml::Value::as_array)
        .and_then(|list| {
            list.iter().find_map(|entry| {
                let from = entry.get("from").and_then(toml::Value::as_str)?;
                let name = entry.get("name").and_then(toml::Value::as_str)?;
                (from == "api.endpoint" && !name.trim().is_empty()).then(|| name.to_string())
            })
        });
    if binding.is_none() {
        wrong(found, "sidecar web must inherit api.endpoint");
    }
    binding
}

fn uses_port(value: Option<&toml::Value>) -> bool {
    value
        .and_then(toml::Value::as_str)
        .is_some_and(|url| url.contains("{port}"))
}

fn api(root: &Path, found: &mut Found) {
    let source = rust_source(&root.join("crates/api/src"));
    if !source.contains("SIDECAR_PORT") {
        wrong(found, "api does not consume SIDECAR_PORT");
    }
    if !source.contains("sidecar_stamp") && !source.contains("sidecar-stamp") {
        wrong(found, "api does not accept --sidecar-stamp");
    }
    if !["\"role\"", "\"api\"", "\"endpoint\""]
        .iter()
        .all(|needle| source.contains(needle))
    {
        wrong(found, "api does not emit api endpoint readiness");
    }
}

fn rust_source(root: &Path) -> String {
    let mut paths = Vec::new();
    collect(root, "rs", &mut paths);
    paths.sort();
    paths
        .iter()
        .filter_map(|path| std::fs::read_to_string(path).ok())
        .collect::<Vec<_>>()
        .join("\n")
}

fn collect(root: &Path, extension: &str, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, extension, found);
        } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
            found.push(path);
        }
    }
}

fn web_plane(root: &Path, package: &Json, inherited: Option<&str>, found: &mut Found) {
    if !has_package(package, "@perish/react-components") {
        wrong(found, "web does not depend on @perish/react-components");
    }
    let plugin = [
        "@perish/vite-plugin-design",
        "@jsr/perish__vite-plugin-design",
    ]
    .iter()
    .any(|name| has_package(package, name));
    if !plugin {
        wrong(found, "web does not depend on @perish/vite-plugin-design");
    }

    let config = [
        "vite.config.ts",
        "vite.config.mts",
        "vite.config.js",
        "vite.config.mjs",
    ]
    .iter()
    .find_map(|name| std::fs::read_to_string(root.join("apps/web").join(name)).ok())
    .unwrap_or_default();
    if !config.contains("SIDECAR_PORT") {
        wrong(found, "vite does not consume SIDECAR_PORT");
    }
    if let Some(binding) = inherited
        && !config.contains(binding)
    {
        wrong(found, &format!("vite does not read inherited {binding}"));
    }
    if !config.contains("design(") {
        wrong(found, "vite does not activate the design plugin");
    }
}

fn wrong(found: &mut Found, line: &str) {
    found.push(("out of true", line.to_string()));
}
