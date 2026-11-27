mod helper;

use helper::Template;
use serde_json::Value as Json;
use std::path::{Path, PathBuf};

pub(crate) struct Evidence {
    pub(crate) env: Vec<(String, String)>,
    pub(crate) mismatch: Vec<String>,
}

pub(crate) struct Client {
    pub(crate) seed: bool,
    pub(crate) map: bool,
    pub(crate) mount: bool,
    pub(crate) checksum: bool,
}

pub(crate) fn read(root: &Path) -> Evidence {
    Tree(root).read()
}

pub(crate) fn templates(root: &Path) -> Vec<(String, String)> {
    Tree(root).templates()
}

pub(crate) fn client(root: &Path, files: &[(String, String)]) -> Client {
    Tree(root).client(files)
}

struct Tree<'a>(&'a Path);

impl Tree<'_> {
    fn read(&self) -> Evidence {
        let mut env = Vec::new();
        let mut mismatch = Vec::new();
        for (path, text) in self.templates() {
            let template = Template(&text);
            for key in template.env() {
                env.push((path.clone(), key));
            }
            if template.mismatch() {
                mismatch.push(path);
            }
        }
        for (path, text) in self.helpers() {
            let template = Template(&text);
            for key in template.env() {
                env.push((path.clone(), key));
            }
            if template.mismatch() || helper::defined(&text) {
                mismatch.push(path);
            }
        }
        env.sort();
        mismatch.sort();
        mismatch.dedup();
        Evidence { env, mismatch }
    }

    fn templates(&self) -> Vec<(String, String)> {
        self.gather(&["yaml", "yml"])
    }

    fn helpers(&self) -> Vec<(String, String)> {
        self.gather(&["tpl"])
    }

    fn gather(&self, extensions: &[&str]) -> Vec<(String, String)> {
        let mut paths = Vec::new();
        let Ok(charts) = std::fs::read_dir(self.0.join("charts")) else {
            return Vec::new();
        };
        for chart in charts.flatten() {
            let dir = chart.path().join("templates");
            for extension in extensions {
                Tree(&dir).collect(extension, &mut paths);
            }
        }
        paths.sort();
        paths
            .into_iter()
            .filter_map(|path| {
                let text = std::fs::read_to_string(&path).ok()?;
                let relative = path
                    .strip_prefix(self.0)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                Some((relative, text))
            })
            .collect()
    }

    fn client(&self, files: &[(String, String)]) -> Client {
        let seed = std::fs::read_to_string(self.0.join("apps/web/public/client.json"))
            .ok()
            .and_then(|text| serde_json::from_str::<Json>(&text).ok())
            .is_some_and(|value| matches!(value, Json::Object(map) if map.is_empty()));
        let mut map = false;
        let mut mount = false;
        let mut checksum = false;
        for (_, text) in files {
            let template = Template(text);
            map |= template.configmap();
            mount |= template.mount();
            checksum |= template.checksum();
        }
        Client {
            seed,
            map,
            mount,
            checksum,
        }
    }

    fn collect(&self, extension: &str, found: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(self.0) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                Tree(&path).collect(extension, found);
            } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
                found.push(path);
            }
        }
    }
}
