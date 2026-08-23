use super::{Image, Plane, Production};
use std::path::{Path, PathBuf};

pub(super) fn read(root: &Path) -> Production {
    let production = Reader(root);
    let image = root.join("deploy/web.Dockerfile");
    let web = if !image.is_file() {
        Image::Absent
    } else {
        match std::fs::read_to_string(image) {
            Ok(text) => Image::Held(Plane {
                runtime: text.contains("dist/.perish/server.mjs"),
                dispatch: !Manifest(&text).proxy(),
            }),
            Err(_) => Image::Unread,
        }
    };
    let templates = production.templates();
    let api = templates
        .iter()
        .any(|text| Manifest(text).workload() && Manifest(text).role("api"));
    let role = templates
        .iter()
        .any(|text| Manifest(text).workload() && Manifest(text).role("web"));
    let ingress = templates
        .iter()
        .any(|text| Manifest(text).ingress("/api", "api") && Manifest(text).ingress("/", "web"));
    let cargo = production.version();
    let aligned = production.charts().iter().any(|path| {
        let Ok(text) = std::fs::read_to_string(path) else {
            return false;
        };
        let version = Manifest(&text).value("version");
        let app = Manifest(&text).value("appVersion");
        cargo.as_deref() == version.as_deref() && version == app
    });
    Production {
        api: root.join("deploy/api.Dockerfile").is_file(),
        web,
        workloads: api && role,
        ingress,
        aligned,
    }
}

struct Reader<'a>(&'a Path);

impl Reader<'_> {
    fn templates(&self) -> Vec<String> {
        let mut paths = Vec::new();
        let Ok(charts) = std::fs::read_dir(self.0.join("charts")) else {
            return Vec::new();
        };
        for chart in charts.flatten() {
            collect(&chart.path().join("templates"), "yaml", &mut paths);
            collect(&chart.path().join("templates"), "yml", &mut paths);
        }
        paths.sort();
        paths
            .iter()
            .filter_map(|path| std::fs::read_to_string(path).ok())
            .collect()
    }

    fn charts(&self) -> Vec<PathBuf> {
        let Ok(charts) = std::fs::read_dir(self.0.join("charts")) else {
            return Vec::new();
        };
        charts
            .flatten()
            .map(|entry| entry.path().join("Chart.yaml"))
            .filter(|path| path.is_file())
            .collect()
    }

    fn version(&self) -> Option<String> {
        let text = std::fs::read_to_string(self.0.join("Cargo.toml")).ok()?;
        let doc = text.parse::<toml::Table>().ok()?;
        doc.get("workspace")
            .and_then(|workspace| workspace.get("package"))
            .and_then(|package| package.get("version"))
            .or_else(|| {
                doc.get("package")
                    .and_then(|package| package.get("version"))
            })
            .and_then(toml::Value::as_str)
            .map(str::to_string)
    }
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

struct Manifest<'a>(&'a str);

impl Manifest<'_> {
    fn value(&self, key: &str) -> Option<String> {
        self.0.lines().find_map(|line| {
            let (held, value) = line.split_once(':')?;
            (held.trim() == key).then(|| value.trim().trim_matches(['"', '\'']).to_string())
        })
    }

    fn workload(&self) -> bool {
        self.0.contains("kind: Deployment") || self.0.contains("kind: StatefulSet")
    }

    fn role(&self, name: &str) -> bool {
        self.0.contains(&format!("-{name}")) || self.0.contains(&format!("name: {name}"))
    }

    fn proxy(&self) -> bool {
        let text = self.0.to_ascii_lowercase();
        text.contains("nginx") || text.contains("api_upstream") || text.contains("proxy_pass")
    }

    fn ingress(&self, path: &str, target: &str) -> bool {
        if !self.0.contains("kind: Ingress") {
            return false;
        }
        let lines = self.0.lines().collect::<Vec<_>>();
        lines.iter().enumerate().any(|(index, line)| {
            if route(line) != Some(path) {
                return false;
            }
            let end = lines[index + 1..]
                .iter()
                .position(|line| route(line).is_some())
                .map_or(lines.len(), |offset| index + 1 + offset);
            Manifest(&lines[index..end].join("\n")).role(target)
        })
    }
}

fn route(line: &str) -> Option<&str> {
    let line = line.trim().strip_prefix("- ").unwrap_or(line.trim());
    let (key, value) = line.split_once(':')?;
    (key.trim() == "path").then(|| value.trim().trim_matches(['"', '\'']))
}
