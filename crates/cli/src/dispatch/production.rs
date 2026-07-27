use super::{Found, wrong};
use std::path::{Path, PathBuf};

pub(super) fn read(root: &Path, found: &mut Found) {
    if !root.join("deploy/api.Dockerfile").is_file() {
        wrong(found, "production has no api image seat");
    }
    if !root.join("deploy/web.Dockerfile").is_file() {
        wrong(found, "production has no web image seat");
    }

    let templates = chart_templates(root);
    let api = templates
        .iter()
        .any(|text| workload(text) && role(text, "api"));
    let web = templates
        .iter()
        .any(|text| workload(text) && role(text, "web"));
    if !api || !web {
        wrong(found, "chart does not split api and web workloads");
    }
    let ingress = templates
        .iter()
        .any(|text| text.contains("kind: Ingress") && role(text, "web"));
    if !ingress {
        wrong(found, "chart ingress does not target web");
    }

    let cargo = cargo_version(root);
    let aligned = chart_files(root).iter().any(|path| {
        let Ok(text) = std::fs::read_to_string(path) else {
            return false;
        };
        let version = yaml_value(&text, "version");
        let app = yaml_value(&text, "appVersion");
        cargo.as_deref() == version.as_deref() && version == app
    });
    if !aligned {
        wrong(found, "Cargo and chart do not share one version train");
    }
}

fn chart_templates(root: &Path) -> Vec<String> {
    let mut paths = Vec::new();
    let Ok(charts) = std::fs::read_dir(root.join("charts")) else {
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

fn chart_files(root: &Path) -> Vec<PathBuf> {
    let Ok(charts) = std::fs::read_dir(root.join("charts")) else {
        return Vec::new();
    };
    charts
        .flatten()
        .map(|entry| entry.path().join("Chart.yaml"))
        .filter(|path| path.is_file())
        .collect()
}

fn cargo_version(root: &Path) -> Option<String> {
    let text = std::fs::read_to_string(root.join("Cargo.toml")).ok()?;
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

fn yaml_value(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let (held, value) = line.split_once(':')?;
        (held.trim() == key).then(|| value.trim().trim_matches(['"', '\'']).to_string())
    })
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

fn workload(text: &str) -> bool {
    text.contains("kind: Deployment") || text.contains("kind: StatefulSet")
}

fn role(text: &str, name: &str) -> bool {
    text.contains(&format!("-{name}")) || text.contains(&format!("name: {name}"))
}
