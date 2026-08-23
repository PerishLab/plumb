use serde_json::Value as Json;
use std::path::{Path, PathBuf};

pub struct Evidence {
    pub(crate) plane: Plane,
    pub(crate) roles: Vec<Role>,
}

pub(crate) struct Plane {
    pub(crate) design: bool,
    pub(crate) plugin: bool,
    pub(crate) dispatch: bool,
    pub(crate) loaded: bool,
    pub(crate) rendered: bool,
    pub(crate) typed: bool,
    pub(crate) build: bool,
    pub(crate) guarded: bool,
}

pub(crate) enum Role {
    Segment,
    View,
    Direct,
    Case,
    Component,
    Hook,
}

pub fn read(root: &Path) -> Option<Evidence> {
    Web(root).read()
}

struct Web<'a>(&'a Path);

impl Web<'_> {
    fn read(&self) -> Option<Evidence> {
        let package = self.package()?;
        if !has(&package, "svelte") || !has(&package, "vite") {
            return None;
        }
        let config = [
            "vite.config.ts",
            "vite.config.mts",
            "vite.config.js",
            "vite.config.mjs",
        ]
        .iter()
        .find_map(|name| std::fs::read_to_string(self.0.join("apps/web").join(name)).ok())
        .unwrap_or_default();
        let source = self.sources(&self.0.join("apps/web/src"));
        let guard = super::operator::Operator(self.0).guard().source;
        let name = package.get("name").and_then(Json::as_str);
        let plane = Plane {
            design: has(&package, "@perish/design"),
            plugin: config.contains("design("),
            dispatch: !config.contains("SIDECAR_PORT") && !config.contains("API_URL"),
            loaded: source.contains("virtual:perish/views"),
            rendered: source.contains("Views") && source.contains("@perish/design"),
            typed: source.contains("declare module \"virtual:perish/views\""),
            build: package
                .get("scripts")
                .and_then(|scripts| scripts.get("build"))
                .and_then(Json::as_str)
                .is_some(),
            guarded: guard.contains("build") && name.is_some_and(|name| guard.contains(name)),
        };
        let mut roles = Vec::new();
        self.roles(&mut roles);
        Some(Evidence { plane, roles })
    }

    fn package(&self) -> Option<Json> {
        let text = std::fs::read_to_string(self.0.join("apps/web/package.json")).ok()?;
        serde_json::from_str(&text).ok()
    }

    fn roles(&self, found: &mut Vec<Role>) {
        let source = self.0.join("apps/web/src");
        self.views(&source.join("views"), found);
        self.direct(&source.join("lib"), found);
        self.components(&source.join("lib/components"), found);
        self.hooks(&source.join("lib/hooks"), found);
    }

    fn views(&self, root: &Path, found: &mut Vec<Role>) {
        let Ok(entries) = std::fs::read_dir(root) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if path.is_dir() {
                if !segment(&name) {
                    found.push(Role::Segment);
                }
                self.views(&path, found);
                continue;
            }
            if path.extension().and_then(|value| value.to_str()) != Some("svelte") {
                found.push(Role::View);
                continue;
            }
            let stem = path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("");
            if stem != "index" && !segment(stem) {
                found.push(Role::Segment);
            }
        }
    }

    fn direct(&self, root: &Path, found: &mut Vec<Role>) {
        let Ok(entries) = std::fs::read_dir(root) else {
            return;
        };
        if entries.flatten().any(|entry| {
            entry.path().extension().and_then(|value| value.to_str()) == Some("svelte")
        }) {
            found.push(Role::Direct);
        }
    }

    fn components(&self, root: &Path, found: &mut Vec<Role>) {
        let Ok(entries) = std::fs::read_dir(root) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name != name.to_ascii_lowercase() {
                found.push(Role::Case);
            }
            if path.is_dir() {
                self.components(&path, found);
            } else if path.extension().and_then(|value| value.to_str()) != Some("svelte") {
                found.push(Role::Component);
            }
        }
    }

    fn hooks(&self, root: &Path, found: &mut Vec<Role>) {
        let Ok(entries) = std::fs::read_dir(root) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name != name.to_ascii_lowercase() {
                found.push(Role::Case);
            }
            if path.is_dir() {
                self.hooks(&path, found);
            } else {
                let stem = path
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or("");
                if path.extension().and_then(|value| value.to_str()) != Some("ts")
                    || stem.is_empty()
                {
                    found.push(Role::Hook);
                }
            }
        }
    }

    fn sources(&self, root: &Path) -> String {
        let mut paths = Vec::new();
        self.collect(root, &mut paths);
        paths.sort();
        paths
            .iter()
            .filter_map(|path| std::fs::read_to_string(path).ok())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn collect(&self, root: &Path, found: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(root) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                self.collect(&path, found);
            } else if matches!(
                path.extension().and_then(|value| value.to_str()),
                Some("ts" | "tsx")
            ) {
                found.push(path);
            }
        }
    }
}

fn has(doc: &Json, name: &str) -> bool {
    [
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ]
    .iter()
    .any(|seat| doc.get(seat).and_then(|map| map.get(name)).is_some())
}

#[rustfmt::skip] fn segment(name: &str) -> bool {
    if let Some(held) = name.strip_prefix('{').and_then(|value| value.strip_suffix('}')) {
        return word(held) && held.as_bytes()[0].is_ascii_lowercase();
    }
    name.split('-').all(word)
}

#[rustfmt::skip] fn word(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
}
