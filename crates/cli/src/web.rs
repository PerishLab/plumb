use serde_json::Value as Json;
use std::path::{Path, PathBuf};

type Found = Vec<(&'static str, String)>;

pub fn read(root: &Path) -> Option<Found> {
    Web(root).read()
}

struct Web<'a>(&'a Path);

impl Web<'_> {
    fn read(&self) -> Option<Found> {
        let package = self.package()?;
        if !has(&package, "react") || !has(&package, "vite") {
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
        if !has(package, "@perish/react-components") {
            wrong(found, "web does not depend on @perish/react-components");
        }
        if ![
            "@perish/vite-plugin-design",
            "@jsr/perish__vite-plugin-design",
        ]
        .iter()
        .any(|name| has(package, name))
        {
            wrong(found, "web does not depend on @perish/vite-plugin-design");
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
            wrong(found, "vite does not activate the design plugin");
        }
        if config.contains("SIDECAR_PORT") || config.contains("API_URL") {
            wrong(found, "vite manually consumes sidecar dispatch environment");
        }

        let source = self.sources(&self.0.join("apps/web/src"));
        if !source.contains("virtual:perish/views") {
            wrong(found, "web does not load the virtual views manifest");
        }
        if !source.contains("Views") || !source.contains("@perish/react-components") {
            wrong(found, "web does not render the views manifest");
        }

        let compiler =
            std::fs::read_to_string(self.0.join("apps/web/tsconfig.json")).unwrap_or_default();
        if !compiler.contains("@perish/react-components/client") {
            wrong(
                found,
                "web compiler does not include @perish/react-components/client",
            );
        }
        if package
            .get("scripts")
            .and_then(|scripts| scripts.get("build"))
            .and_then(Json::as_str)
            .is_none()
        {
            wrong(found, "web has no build script");
        }
        let guard =
            std::fs::read_to_string(self.0.join(".runseal/wrappers/guard.ts")).unwrap_or_default();
        let name = package.get("name").and_then(Json::as_str);
        if !guard.contains("\"build\"") || name.is_none_or(|name| !guard.contains(name)) {
            wrong(found, "guard does not build the web app");
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
                    wrong(found, "web view paths must be lowercase route segments");
                }
                self.views(&path, found);
                continue;
            }
            if path.extension().and_then(|value| value.to_str()) != Some("tsx") {
                wrong(found, "web views only hold route tsx files");
                continue;
            }
            let stem = path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("");
            if stem != "index" && !segment(stem) {
                wrong(found, "web view paths must be lowercase route segments");
            }
        }
    }

    fn direct(&self, root: &Path, found: &mut Found) {
        let Ok(entries) = std::fs::read_dir(root) else {
            return;
        };
        if entries
            .flatten()
            .any(|entry| entry.path().extension().and_then(|value| value.to_str()) == Some("tsx"))
        {
            wrong(found, "web lib tsx must live under lib/components");
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
                wrong(found, "web convention paths must be lowercase");
            }
            if path.is_dir() {
                self.components(&path, found);
            } else if path.extension().and_then(|value| value.to_str()) != Some("tsx") {
                wrong(found, "web components only hold lowercase tsx files");
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
                wrong(found, "web convention paths must be lowercase");
            }
            if path.is_dir() {
                self.hooks(&path, found);
            } else {
                let stem = path
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or("");
                if path.extension().and_then(|value| value.to_str()) != Some("ts")
                    || !stem.starts_with("use-")
                {
                    wrong(found, "web hooks must be lowercase use-*.ts files");
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

fn segment(name: &str) -> bool {
    if let Some(held) = name
        .strip_prefix('{')
        .and_then(|name| name.strip_suffix('}'))
    {
        return !held.is_empty()
            && held.chars().enumerate().all(|(index, value)| {
                value.is_ascii_lowercase() || (index > 0 && value.is_ascii_digit())
            });
    }
    name.split('-').all(|part| {
        !part.is_empty()
            && part
                .chars()
                .all(|value| value.is_ascii_lowercase() || value.is_ascii_digit())
    })
}

fn wrong(found: &mut Found, line: &str) {
    found.push(("out of true", line.to_string()));
}
