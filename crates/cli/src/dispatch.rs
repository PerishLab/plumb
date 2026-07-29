use crate::judge::catalog::model::Mechanism;
use crate::judge::catalog::rules::dispatch as rule;
use crate::judge::finding::{Found, Seed};
use serde_json::Value as Json;
use std::path::{Path, PathBuf};

mod production;

pub fn read(root: &Path) -> Option<Found> {
    let workspace = Workspace(root);
    let web = workspace.package()?;
    if !contains(&web, "react") || !contains(&web, "vite") || !workspace.executable() {
        return None;
    }

    let mut found = Found::new();
    workspace.sidecar(&mut found);
    workspace.api(&mut found);
    production::read(root, &mut found);
    Some(found)
}

struct Workspace<'a>(&'a Path);

fn contains(doc: &Json, name: &str) -> bool {
    [
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ]
    .iter()
    .any(|seat| doc.get(seat).and_then(|map| map.get(name)).is_some())
}

impl Workspace<'_> {
    fn package(&self) -> Option<Json> {
        let text = std::fs::read_to_string(self.0.join("apps/web/package.json")).ok()?;
        serde_json::from_str(&text).ok()
    }

    fn executable(&self) -> bool {
        let seat = self.0.join("crates/api");
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

    fn sidecar(&self, found: &mut Found) {
        let path = self.0.join("sidecar.toml");
        let Ok(text) = std::fs::read_to_string(&path) else {
            wrong(
                found,
                &rule::SIDECAR_MANIFEST_PRESENT,
                "web/api pair has no sidecar.toml",
            );
            return;
        };
        let doc = match text.parse::<toml::Table>() {
            Ok(doc) => toml::Value::Table(doc),
            Err(error) => {
                found.push(Seed::blind(
                    &rule::SIDECAR_MANIFEST_READABLE,
                    format!(
                        "cannot read sidecar.toml: {}",
                        error.to_string().lines().next().unwrap_or("")
                    ),
                ));
                return;
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
            None => wrong(
                found,
                &rule::API_ROLE_DECLARED,
                "sidecar does not declare the api role",
            ),
            Some(api) => {
                if api.get("port").and_then(toml::Value::as_integer) != Some(0) {
                    wrong(
                        found,
                        &rule::API_PORT_LEASED,
                        "sidecar api must lease port 0",
                    );
                }
                if api
                    .get("ready")
                    .and_then(|ready| ready.get("role"))
                    .and_then(toml::Value::as_str)
                    != Some("api")
                {
                    wrong(
                        found,
                        &rule::API_READY_ROLE,
                        "sidecar api must declare ready role api",
                    );
                }
                if !port(api.get("health_url"), Some("/api/health")) {
                    wrong(
                        found,
                        &rule::API_HEALTH_ROUTE,
                        "sidecar api health_url must target {port}/api/health",
                    );
                }
            }
        }

        let app = doc.get("app");
        if app
            .and_then(|app| app.get("name"))
            .and_then(toml::Value::as_str)
            != Some("web")
        {
            wrong(
                found,
                &rule::WEB_APP_DECLARED,
                "sidecar does not declare the web app",
            );
        }
        if app
            .and_then(|app| app.get("port"))
            .and_then(toml::Value::as_integer)
            != Some(0)
        {
            wrong(
                found,
                &rule::WEB_PORT_LEASED,
                "sidecar web must lease port 0",
            );
        }
        if !port(app.and_then(|app| app.get("health_url")), None) {
            wrong(
                found,
                &rule::WEB_HEALTH_PORT,
                "sidecar web health_url must use {port}",
            );
        }

        let binding = app
            .and_then(|app| app.get("inherits_env"))
            .and_then(toml::Value::as_array)
            .is_some_and(|list| {
                list.iter().any(|entry| {
                    entry.get("from").and_then(toml::Value::as_str) == Some("api.endpoint")
                        && entry.get("name").and_then(toml::Value::as_str) == Some("API_URL")
                })
            });
        if !binding {
            wrong(
                found,
                &rule::WEB_INHERITS_API_ENDPOINT,
                "sidecar web must inherit api.endpoint as API_URL",
            );
        }
    }

    fn api(&self, found: &mut Found) {
        let source = self.source(&self.0.join("crates/api/src"), "rs");
        let launch = std::fs::read_to_string(self.0.join("sidecar.toml")).unwrap_or_default();
        if !source.contains("SIDECAR_PORT") && !launch.contains("SIDECAR_PORT") && !mapping(&launch)
        {
            wrong(
                found,
                &rule::API_CONSUMES_PORT,
                "api does not consume SIDECAR_PORT",
            );
        }
        if !source.contains("sidecar_stamp") && !source.contains("sidecar-stamp") {
            wrong(
                found,
                &rule::API_ACCEPTS_STAMP,
                "api does not accept --sidecar-stamp",
            );
        }
        if !["\"role\"", "\"api\"", "\"endpoint\""]
            .iter()
            .all(|needle| source.contains(needle))
        {
            wrong(
                found,
                &rule::API_EMITS_READINESS,
                "api does not emit api endpoint readiness",
            );
        }
        if !source.contains("\"/api\"") || !source.contains(".nest(") {
            wrong(
                found,
                &rule::API_NAMESPACE_MOUNTED,
                "api does not mount the /api namespace",
            );
        }
    }

    fn source(&self, root: &Path, extension: &str) -> String {
        let mut paths = Vec::new();
        collect(root, extension, &mut paths);
        paths.sort();
        paths
            .iter()
            .filter_map(|path| std::fs::read_to_string(path).ok())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn port(value: Option<&toml::Value>, path: Option<&str>) -> bool {
    value
        .and_then(toml::Value::as_str)
        .is_some_and(|url| url.contains("{port}") && path.is_none_or(|path| url.ends_with(path)))
}

fn mapping(launch: &str) -> bool {
    let Ok(doc) = launch.parse::<toml::Table>() else {
        return false;
    };
    doc.get("sidecars")
        .and_then(toml::Value::as_array)
        .and_then(|list| {
            list.iter()
                .find(|entry| entry.get("name").and_then(toml::Value::as_str) == Some("api"))
        })
        .and_then(|api| api.get("env"))
        .and_then(toml::Value::as_table)
        .is_some_and(|env| env.values().any(|value| value.as_str() == Some("{port}")))
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

fn wrong(found: &mut Found, rule: &'static Mechanism, evidence: &str) {
    found.push(Seed::wrong(rule, evidence));
}
