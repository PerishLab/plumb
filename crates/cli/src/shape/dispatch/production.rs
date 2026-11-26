use super::{Image, Plane, Production, chart};
use crate::shape::release::{Input, Spec};
use std::path::{Path, PathBuf};

pub(super) fn read(root: &Path) -> Production {
    let production = Reader(root);
    let spec = Spec::resolve(root).ok();
    let placed = spec.as_ref().is_some_and(|spec| spec.cfworker.is_some());
    let parts = spec
        .as_ref()
        .and_then(|spec| spec.oci.as_ref().map(|oci| oci.parts(spec)))
        .unwrap_or_default();
    let contained = parts.iter().any(|part| {
        matches!(part.input, Input::Binary(_) | Input::Source) && root.join(&part.file).is_file()
    });
    let webs: Vec<PathBuf> = parts
        .iter()
        .filter(|part| matches!(part.input, Input::Web(_)))
        .map(|part| root.join(&part.file))
        .collect();
    let web = if placed {
        Image::Placed
    } else if webs.is_empty() {
        plane(&[root.join("deploy/web.Dockerfile")])
    } else {
        plane(&webs)
    };
    let files = chart::templates(root);
    let api = files
        .iter()
        .any(|(_, text)| Manifest(text).workload() && Manifest(text).role("api"));
    let role = placed
        || files
            .iter()
            .any(|(_, text)| Manifest(text).workload() && Manifest(text).role("web"));
    let ingress = files.iter().any(|(_, text)| {
        Manifest(text).ingress("/api", "api") && (placed || Manifest(text).ingress("/", "web"))
    });
    let cargo = production.version();
    let charts = production.charts();
    let aligned = charts.iter().any(|path| {
        let Ok(text) = std::fs::read_to_string(path) else {
            return false;
        };
        let version = Manifest(&text).value("version");
        let app = Manifest(&text).value("appVersion");
        cargo.as_deref() == version.as_deref() && version == app
    });
    let client =
        (!placed && !webs.is_empty() && !charts.is_empty()).then(|| chart::client(root, &files));
    Production {
        api: contained || root.join("deploy/api.Dockerfile").is_file(),
        web,
        workloads: api && role,
        ingress,
        aligned,
        client,
    }
}

fn plane(paths: &[PathBuf]) -> Image {
    let mut runtime = true;
    let mut dispatch = true;
    for path in paths {
        if !path.is_file() {
            return Image::Absent;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            return Image::Unread;
        };
        runtime &= text.contains("dist/.perish/server.mjs");
        dispatch &= !Manifest(&text).proxy();
    }
    Image::Held(Plane { runtime, dispatch })
}

struct Reader<'a>(&'a Path);

impl Reader<'_> {
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
