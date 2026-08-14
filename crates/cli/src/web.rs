use crate::judge::catalog::model::Mechanism;
use crate::judge::catalog::rules::web as rule;
use crate::judge::finding::{Found, Seed};
use serde_json::Value as Json;
use std::path::{Path, PathBuf};

pub fn read(root: &Path) -> Option<Found> {
    Web(root).read()
}

struct Web<'a>(&'a Path);

impl Web<'_> {
    fn read(&self) -> Option<Found> {
        let package = self.package()?;
        if !has(&package, "svelte") || !has(&package, "vite") {
            return None;
        }
        let mut found = Found::new();
        self.plane(&package, &mut found);
        self.roles(&mut found);
        Some(found)
    }

    fn package(&self) -> Option<Json> {
        let text = std::fs::read_to_string(self.0.join("apps/web/package.json")).ok()?;
        serde_json::from_str(&text).ok()
    }

    fn plane(&self, package: &Json, found: &mut Found) {
        if !has(package, "@perish/design") {
            wrong(
                found,
                &rule::DESIGN_DEPENDENCY,
                "web does not depend on @perish/design",
            );
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
        if !config.contains("design(") {
            wrong(
                found,
                &rule::DESIGN_PLUGIN_ACTIVE,
                "vite does not activate the design plugin",
            );
        }
        if config.contains("SIDECAR_PORT") || config.contains("API_URL") {
            wrong(
                found,
                &rule::VITE_DOES_NOT_OWN_DISPATCH,
                "vite manually consumes sidecar dispatch environment",
            );
        }
        let source = self.sources(&self.0.join("apps/web/src"));
        if !source.contains("virtual:perish/views") {
            wrong(
                found,
                &rule::VIEWS_MANIFEST_LOADED,
                "web does not load the virtual views manifest",
            );
        }
        if !source.contains("Views") || !source.contains("@perish/design") {
            wrong(
                found,
                &rule::VIEWS_MANIFEST_RENDERED,
                "web does not render the views manifest",
            );
        }

        if !source.contains("declare module \"virtual:perish/views\"") {
            wrong(
                found,
                &rule::VIEWS_TYPES_DECLARED,
                "web does not declare the virtual views module type",
            );
        }
        if package
            .get("scripts")
            .and_then(|scripts| scripts.get("build"))
            .and_then(Json::as_str)
            .is_none()
        {
            wrong(
                found,
                &rule::BUILD_SCRIPT_PRESENT,
                "web has no build script",
            );
        }
        let guard = crate::shape::operator::Operator(self.0).guard().source;
        let name = package.get("name").and_then(Json::as_str);
        if !guard.contains("build") || name.is_none_or(|name| !guard.contains(name)) {
            wrong(
                found,
                &rule::GUARD_BUILDS_WEB,
                "guard does not build the web app",
            );
        }
    }

    fn roles(&self, found: &mut Found) {
        let source = self.0.join("apps/web/src");
        self.views(&source.join("views"), found);
        self.direct(&source.join("lib"), found);
        self.components(&source.join("lib/components"), found);
        self.hooks(&source.join("lib/hooks"), found);
    }

    fn views(&self, root: &Path, found: &mut Found) {
        let Ok(entries) = std::fs::read_dir(root) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if path.is_dir() {
                if !segment(&name) {
                    wrong(
                        found,
                        &rule::VIEW_PATH_SEGMENT,
                        "web view paths must be lowercase route segments",
                    );
                }
                self.views(&path, found);
                continue;
            }
            if path.extension().and_then(|value| value.to_str()) != Some("svelte") {
                wrong(
                    found,
                    &rule::VIEW_FILE_KIND,
                    "web views only hold route svelte files",
                );
                continue;
            }
            let stem = path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("");
            if stem != "index" && !segment(stem) {
                wrong(
                    found,
                    &rule::VIEW_PATH_SEGMENT,
                    "web view paths must be lowercase route segments",
                );
            }
        }
    }

    fn direct(&self, root: &Path, found: &mut Found) {
        let Ok(entries) = std::fs::read_dir(root) else {
            return;
        };
        if entries.flatten().any(|entry| {
            entry.path().extension().and_then(|value| value.to_str()) == Some("svelte")
        }) {
            wrong(
                found,
                &rule::SVELTE_UNDER_COMPONENTS,
                "web lib svelte must live under lib/components",
            );
        }
    }

    fn components(&self, root: &Path, found: &mut Found) {
        let Ok(entries) = std::fs::read_dir(root) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name != name.to_ascii_lowercase() {
                wrong(
                    found,
                    &rule::CONVENTION_PATH_LOWERCASE,
                    "web convention paths must be lowercase",
                );
            }
            if path.is_dir() {
                self.components(&path, found);
            } else if path.extension().and_then(|value| value.to_str()) != Some("svelte") {
                wrong(
                    found,
                    &rule::COMPONENT_FILE_KIND,
                    "web components only hold lowercase svelte files",
                );
            }
        }
    }

    fn hooks(&self, root: &Path, found: &mut Found) {
        let Ok(entries) = std::fs::read_dir(root) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name != name.to_ascii_lowercase() {
                wrong(
                    found,
                    &rule::CONVENTION_PATH_LOWERCASE,
                    "web convention paths must be lowercase",
                );
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
                    wrong(
                        found,
                        &rule::HOOK_FILE_KIND,
                        "web hooks must be lowercase .ts files",
                    );
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
fn wrong(found: &mut Found, rule: &'static Mechanism, evidence: &str) {
    found.push(Seed::wrong(rule, evidence));
}
