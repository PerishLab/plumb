use super::channel;
use crate::command::operator::topology;
use crate::shape::release::Spec;

pub fn plan(spec: &Spec, reference: &str, commit: &str) -> Result<String, String> {
    let version = topology::reference(reference)?;
    let channel = channel::channel(&version)?;
    topology::bind(topology::Source {
        root: &spec.root,
        channel: &channel,
        version: &version,
        commit,
        reference,
    })?;
    let mut held = media(spec)?;
    let table = held.as_object_mut().ok_or("media is not an object")?;
    table.insert("version".into(), serde_json::json!(version));
    table.insert("channel".into(), serde_json::json!(channel));
    table.insert("authority".into(), serde_json::json!(spec.authority));
    serde_json::to_string(&held).map_err(|error| error.to_string())
}

pub fn surface(spec: &Spec) -> Result<String, String> {
    serde_json::to_string(&media(spec)?).map_err(|error| error.to_string())
}

fn media(spec: &Spec) -> Result<serde_json::Value, String> {
    let held = spec.surface();
    let row = |medium: &&str| serde_json::json!({ "medium": medium });
    let include = held.iter().map(row).collect::<Vec<_>>();
    let mut publication = Vec::new();
    let request = |project| {
        request(
            project,
            spec.configuration.as_deref(),
            spec.profile.as_deref(),
        )
    };
    if spec.cargo.is_some() {
        let mut roots = vec!["Cargo.toml".into(), "crates".into()];
        if spec.root.join("Cargo.lock").is_file() {
            roots.push("Cargo.lock".into());
        }
        if let Some(depends) = spec.depends.get("cargo") {
            roots.extend(
                depends
                    .iter()
                    .map(|path| path.to_string_lossy().to_string()),
            );
        }
        roots.sort();
        roots.dedup();
        publication.push(request(Project {
            action: "ship/cargo".into(),
            projections: Vec::new(),
            roots,
            kind: "cargo",
            package: None,
        }));
    }
    if spec.cfworker.is_some() {
        let mut roots = [
            "Cargo.toml",
            "apps",
            "package.json",
            "packages",
            "pnpm-lock.yaml",
            "pnpm-workspace.yaml",
        ]
        .into_iter()
        .filter(|path| spec.root.join(path).exists())
        .map(str::to_string)
        .collect::<Vec<_>>();
        if let Some(depends) = spec.depends.get("cfworker") {
            roots.extend(
                depends
                    .iter()
                    .map(|path| path.to_string_lossy().to_string()),
            );
        }
        roots.sort();
        roots.dedup();
        publication.push(request(Project {
            action: "ship/cfworker".into(),
            projections: Vec::new(),
            roots,
            kind: "cfworker",
            package: None,
        }));
    }
    if spec.oci.is_some() {
        publication.push(request(Project {
            action: "ship/oci".into(),
            projections: Vec::new(),
            roots: vec!["*".into()],
            kind: "oci",
            package: None,
        }));
    }
    if let Some(chart) = &spec.chart {
        let held = chart.chart.rsplit('/').next().unwrap_or(&chart.chart);
        let mut roots = vec![format!("charts/{held}")];
        if let Some(depends) = spec.depends.get("chart") {
            roots.extend(
                depends
                    .iter()
                    .map(|path| path.to_string_lossy().to_string()),
            );
        }
        roots.sort();
        roots.dedup();
        publication.push(request(Project {
            action: "ship/chart".into(),
            projections: vec![
                format!("charts/{held}/Chart.yaml#/version"),
                format!("charts/{held}/Chart.yaml#/appVersion"),
            ],
            roots,
            kind: "chart",
            package: None,
        }));
    }
    if let Some(npm) = &spec.npm {
        for package in &npm.packages {
            let bare = package.rsplit('/').next().unwrap_or(package);
            let mut roots = vec![format!("packages/{bare}")];
            if spec.root.join("pnpm-lock.yaml").is_file() {
                roots.push("pnpm-lock.yaml".to_string());
            }
            if let Some(depends) = spec.depends.get("npm") {
                roots.extend(
                    depends
                        .iter()
                        .map(|path| path.to_string_lossy().to_string()),
                );
            }
            roots.sort();
            roots.dedup();
            publication.push(request(Project {
                action: format!("ship/npm.{bare}"),
                projections: vec![format!("packages/{bare}/package.json#/version")],
                roots,
                kind: "npm",
                package: Some(package),
            }));
        }
    }
    Ok(serde_json::json!({
        "include": include,
        "publication": { "include": publication },
    }))
}

struct Project<'a> {
    action: String,
    projections: Vec<String>,
    roots: Vec<String>,
    kind: &'a str,
    package: Option<&'a str>,
}

fn request(
    project: Project<'_>,
    configuration: Option<&str>,
    profile: Option<&str>,
) -> serde_json::Value {
    let mut operation = serde_json::json!({ "type": project.kind });
    if let Some(package) = project.package {
        operation["package"] = serde_json::json!(package);
    }
    let mut request = serde_json::json!({
        "schema": "plumb.ship-request/v2",
        "action": project.action,
        "projections": project.projections,
        "roots": project.roots,
        "operation": operation,
    });
    if let Some(configuration) = configuration {
        request["configuration"] = serde_json::json!(configuration);
    }
    if let Some(profile) = profile {
        request["profile"] = serde_json::json!(profile);
    }
    request
}
