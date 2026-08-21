use super::{Found, wrong};
use crate::catalog::rules::dispatch as rule;
use std::path::{Path, PathBuf};

pub(super) fn read(root: &Path, found: &mut Found) {
    let production = Production(root);
    if !root.join("deploy/api.Dockerfile").is_file() {
        wrong(
            found,
            &rule::API_IMAGE_PRESENT,
            "production has no api image seat",
        );
    }
    let image = root.join("deploy/web.Dockerfile");
    if !image.is_file() {
        wrong(
            found,
            &rule::WEB_IMAGE_PRESENT,
            "production has no web image seat",
        );
    } else if let Ok(text) = std::fs::read_to_string(image) {
        if !text.contains("dist/.perish/server.mjs") {
            wrong(
                found,
                &rule::WEB_IMAGE_RUNS_DESIGN_RUNTIME,
                "web image does not run the emitted design runtime",
            );
        }
        if Manifest(&text).proxy() {
            wrong(
                found,
                &rule::WEB_IMAGE_DOES_NOT_OWN_DISPATCH,
                "web image still owns public proxy dispatch",
            );
        }
    }

    let templates = production.templates();
    let api = templates
        .iter()
        .any(|text| Manifest(text).workload() && Manifest(text).role("api"));
    let web = templates
        .iter()
        .any(|text| Manifest(text).workload() && Manifest(text).role("web"));
    if !api || !web {
        wrong(
            found,
            &rule::CHART_SPLITS_WORKLOADS,
            "chart does not split api and web workloads",
        );
    }
    let ingress = templates
        .iter()
        .any(|text| Manifest(text).ingress("/api", "api") && Manifest(text).ingress("/", "web"));
    if !ingress {
        wrong(
            found,
            &rule::CHART_SPLITS_INGRESS,
            "chart ingress does not split /api and / between api and web",
        );
    }

    let cargo = production.version();
    let aligned = production.charts().iter().any(|path| {
        let Ok(text) = std::fs::read_to_string(path) else {
            return false;
        };
        let version = Manifest(&text).value("version");
        let app = Manifest(&text).value("appVersion");
        cargo.as_deref() == version.as_deref() && version == app
    });
    if !aligned {
        wrong(
            found,
            &rule::CARGO_CHART_VERSION_TRAIN,
            "Cargo and chart do not share one version train",
        );
    }
}

struct Production<'a>(&'a Path);

impl Production<'_> {
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
