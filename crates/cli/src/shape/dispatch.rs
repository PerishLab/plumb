mod production;
use serde_json::Value as Json;
use std::path::{Path, PathBuf};

pub struct Evidence {
    pub(crate) manifest: Manifest,
    pub(crate) code: Code,
    pub(crate) production: Production,
}

pub(crate) enum Manifest {
    Absent,
    Unread(String),
    Held(Sidecar),
}

pub(crate) struct Sidecar {
    pub(crate) api: Option<Api>,
    pub(crate) app: App,
}

pub(crate) struct Api {
    pub(crate) port: bool,
    pub(crate) ready: bool,
    pub(crate) health: bool,
}

pub(crate) struct App {
    pub(crate) named: bool,
    pub(crate) port: bool,
    pub(crate) health: bool,
    pub(crate) binding: bool,
}

pub(crate) struct Code {
    pub(crate) port: bool,
    pub(crate) stamp: bool,
    pub(crate) ready: bool,
    pub(crate) namespace: bool,
}

pub(crate) struct Production {
    pub(crate) api: bool,
    pub(crate) web: Image,
    pub(crate) workloads: bool,
    pub(crate) ingress: bool,
    pub(crate) aligned: bool,
}

pub(crate) enum Image {
    Absent,
    Unread,
    Held(Plane),
}

pub(crate) struct Plane {
    pub(crate) runtime: bool,
    pub(crate) dispatch: bool,
}

pub fn read(root: &Path) -> Option<Evidence> {
    let workspace = Workspace(root);
    let web = workspace.package()?;
    if !contains(&web, "svelte") || !contains(&web, "vite") || !workspace.executable() {
        return None;
    }
    Some(Evidence {
        manifest: workspace.sidecar(),
        code: workspace.api(),
        production: production::read(root),
    })
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
        let target = [
            seat.join("src/main.rs").is_file(),
            seat.join("src/bin").is_dir(),
        ]
        .into_iter()
        .any(|held| held)
            || doc.get("bin").and_then(toml::Value::as_array).is_some();
        named && target
    }

    fn sidecar(&self) -> Manifest {
        let path = self.0.join("sidecar.toml");
        let Ok(text) = std::fs::read_to_string(&path) else {
            return Manifest::Absent;
        };
        let doc = match text.parse::<toml::Table>() {
            Ok(doc) => toml::Value::Table(doc),
            Err(error) => {
                return Manifest::Unread(
                    error.to_string().lines().next().unwrap_or("").to_string(),
                );
            }
        };
        let api = doc
            .get("sidecars")
            .and_then(toml::Value::as_array)
            .and_then(|list| {
                list.iter()
                    .find(|entry| entry.get("name").and_then(toml::Value::as_str) == Some("api"))
            })
            .map(|api| Api {
                port: api.get("port").and_then(toml::Value::as_integer) == Some(0),
                ready: api
                    .get("ready")
                    .and_then(|ready| ready.get("role"))
                    .and_then(toml::Value::as_str)
                    == Some("api"),
                health: port(api.get("health_url"), Some("/api/health")),
            });
        let app = doc.get("app");
        let named = app
            .and_then(|app| app.get("name"))
            .and_then(toml::Value::as_str)
            == Some("web");
        let leased = app
            .and_then(|app| app.get("port"))
            .and_then(toml::Value::as_integer)
            == Some(0);
        let health = port(app.and_then(|app| app.get("health_url")), None);
        let binding = app
            .and_then(|app| app.get("inherits_env"))
            .and_then(toml::Value::as_array)
            .is_some_and(|list| {
                list.iter().any(|entry| {
                    entry.get("from").and_then(toml::Value::as_str) == Some("api.endpoint")
                        && entry.get("name").and_then(toml::Value::as_str) == Some("API_URL")
                })
            });
        Manifest::Held(Sidecar {
            api,
            app: App {
                named,
                port: leased,
                health,
                binding,
            },
        })
    }

    fn api(&self) -> Code {
        let source = self.source(&self.0.join("crates/api/src"), "rs");
        let launch = std::fs::read_to_string(self.0.join("sidecar.toml")).unwrap_or_default();
        Code {
            port: source.contains("SIDECAR_PORT")
                || launch.contains("SIDECAR_PORT")
                || mapping(&launch),
            stamp: source.contains("sidecar_stamp") || source.contains("sidecar-stamp"),
            ready: ["\"role\"", "\"api\"", "\"endpoint\""]
                .iter()
                .all(|needle| source.contains(needle)),
            namespace: source.contains("\"/api\"") && source.contains(".nest("),
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
