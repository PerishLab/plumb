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
        Evidence { env, mismatch }
    }

    fn templates(&self) -> Vec<(String, String)> {
        let mut paths = Vec::new();
        let Ok(charts) = std::fs::read_dir(self.0.join("charts")) else {
            return Vec::new();
        };
        for chart in charts.flatten() {
            let dir = chart.path().join("templates");
            Tree(&dir).collect("yaml", &mut paths);
            Tree(&dir).collect("yml", &mut paths);
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

struct Template<'a>(&'a str);

impl Template<'_> {
    fn env(&self) -> Vec<String> {
        let mut found = Vec::new();
        for line in self.0.lines() {
            let Some((key, value)) = entry(line) else {
                continue;
            };
            if key == "name" && legacy(value) {
                found.push(value.to_string());
            }
        }
        found
    }

    fn mismatch(&self) -> bool {
        let lines: Vec<&str> = self.0.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            let Some((key, value)) = entry(line) else {
                continue;
            };
            if key != "securityContext" || !value.is_empty() {
                continue;
            }
            if block(&lines[index + 1..], indent(line)) {
                return true;
            }
        }
        false
    }

    fn configmap(&self) -> bool {
        documents(self.0).any(|doc| {
            let mut kind = false;
            let mut key = false;
            for line in doc.lines() {
                if let Some((name, value)) = entry(line) {
                    kind |= name == "kind" && value == "ConfigMap";
                    key |= name == "client.json";
                }
            }
            kind && key
        })
    }

    fn mount(&self) -> bool {
        let lines: Vec<&str> = self.0.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            let Some((key, value)) = entry(line) else {
                continue;
            };
            if key != "volumeMounts" || !value.is_empty() {
                continue;
            }
            if mounted(&lines[index + 1..], indent(line)) {
                return true;
            }
        }
        false
    }

    fn checksum(&self) -> bool {
        self.0.lines().any(|line| {
            entry(line).is_some_and(|(key, value)| key == "checksum/client" && !value.is_empty())
        })
    }
}

fn documents(text: &str) -> impl Iterator<Item = &str> {
    text.split("\n---")
}

fn block(lines: &[&str], base: usize) -> bool {
    let mut group = false;
    let mut policy = false;
    for line in lines {
        let trimmed = strip(line).trim();
        if trimmed.is_empty() {
            continue;
        }
        let depth = indent(line);
        if directive(trimmed) {
            if depth <= base && trimmed.contains("end") {
                break;
            }
            continue;
        }
        if depth <= base {
            break;
        }
        if let Some((key, value)) = entry(line) {
            group |= key == "fsGroup";
            policy |= key == "fsGroupChangePolicy" && value == "OnRootMismatch";
        }
    }
    group && !policy
}

fn mounted(lines: &[&str], base: usize) -> bool {
    let mut path = false;
    let mut sub = false;
    let mut readonly = false;
    for line in lines {
        let trimmed = strip(line).trim();
        if trimmed.is_empty() {
            continue;
        }
        let depth = indent(line);
        if directive(trimmed) {
            if depth <= base && trimmed.contains("end") {
                break;
            }
            continue;
        }
        if depth <= base {
            break;
        }
        if trimmed.starts_with("- ") {
            if path && sub && readonly {
                return true;
            }
            path = false;
            sub = false;
            readonly = false;
        }
        if let Some((key, value)) = entry(line) {
            path |= key == "mountPath" && value == "/web/dist/client.json";
            sub |= key == "subPath" && value == "client.json";
            readonly |= key == "readOnly" && value == "true";
        }
    }
    path && sub && readonly
}

fn legacy(value: &str) -> bool {
    value.starts_with("API_")
        && value.len() > 4
        && value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
}

fn entry(line: &str) -> Option<(&str, &str)> {
    let trimmed = strip(line).trim();
    if trimmed.is_empty() || directive(trimmed) {
        return None;
    }
    let body = trimmed.strip_prefix("- ").unwrap_or(trimmed);
    let (key, value) = body.split_once(':')?;
    let key = key.trim();
    let value = value.trim().trim_matches(['"', '\'']);
    (!key.is_empty()).then_some((key, value))
}

fn strip(line: &str) -> &str {
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') {
        return "";
    }
    line.split(" #").next().unwrap_or(line)
}

fn directive(trimmed: &str) -> bool {
    trimmed.starts_with("{{") && trimmed.ends_with("}}")
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}
