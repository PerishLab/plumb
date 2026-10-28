use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Deserialize)]
pub(super) struct Metadata {
    pub packages: Vec<Package>,
    #[serde(rename = "workspace_members")]
    pub workspace: Vec<String>,
    pub resolve: Option<Resolve>,
}

#[derive(Clone, Deserialize)]
pub(super) struct Package {
    pub id: String,
    pub name: String,
    pub version: String,
    pub source: Option<String>,
    #[serde(rename = "manifest_path")]
    pub manifest: PathBuf,
    pub dependencies: Vec<Declared>,
    pub targets: Vec<Target>,
}

#[derive(Clone, Deserialize)]
pub(super) struct Declared {
    pub name: String,
    pub rename: Option<String>,
    pub source: Option<String>,
    pub req: String,
    pub path: Option<PathBuf>,
    pub kind: Option<String>,
    pub target: Option<String>,
    pub optional: bool,
    pub features: Vec<String>,
    #[serde(rename = "uses_default_features")]
    pub default: bool,
}

#[derive(Clone, Deserialize)]
pub(super) struct Target {
    pub kind: Vec<String>,
    pub name: Option<String>,
}

#[derive(Clone, Deserialize)]
pub(super) struct Resolve {
    pub nodes: Vec<Resolved>,
}

#[derive(Clone, Deserialize)]
pub(super) struct Resolved {
    pub id: String,
    pub features: Vec<String>,
    pub deps: Vec<Dependency>,
}

#[derive(Clone, Deserialize)]
pub(super) struct Dependency {
    pub name: String,
    pub pkg: String,
    #[serde(rename = "dep_kinds")]
    pub kinds: Vec<Condition>,
}

#[derive(Clone, Deserialize)]
pub(super) struct Condition {
    pub kind: Option<String>,
    pub target: Option<String>,
}

#[derive(Clone, Serialize)]
pub(crate) struct Node {
    pub id: String,
    pub name: String,
    pub version: Option<String>,
    pub source: Option<String>,
    pub domain: bool,
    pub workspace: bool,
    pub manifest: Option<PathBuf>,
    pub features: Vec<String>,
    pub aliases: Vec<String>,
}

#[derive(Clone, Serialize)]
pub(crate) struct Edge {
    pub from: String,
    pub to: String,
    pub alias: String,
    pub requirement: String,
    pub source: Option<String>,
    pub kind: String,
    pub target: Option<String>,
    pub optional: bool,
    pub features: Vec<String>,
    pub default: bool,
    pub resolved: bool,
    pub propagated: bool,
    pub seat: PathBuf,
}

#[derive(Clone, Serialize)]
pub(crate) struct Refusal {
    pub reason: String,
    pub path: Vec<Edge>,
}

#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct Binding {
    alias: String,
    requirement: String,
    kind: String,
    target: Option<String>,
    source: Option<String>,
}

impl Edge {
    pub(super) fn binding(&self) -> Binding {
        Binding {
            alias: self.alias.clone(),
            requirement: self.requirement.clone(),
            kind: self.kind.clone(),
            target: self.target.clone(),
            source: self.source.clone(),
        }
    }
}

impl Declared {
    pub fn alias(&self) -> String {
        self.rename
            .as_deref()
            .unwrap_or(&self.name)
            .replace('-', "_")
    }

    pub fn kind(&self) -> &str {
        self.kind.as_deref().unwrap_or("normal")
    }

    pub fn agrees(&self, dependency: &Dependency, condition: &Condition, target: &Node) -> bool {
        if self.kind != condition.kind || self.target != condition.target {
            return false;
        }
        if self.name != target.name || self.source != target.source {
            return false;
        }
        match self.rename.as_ref() {
            Some(_) => self.alias() == dependency.name,
            None => self.alias() == dependency.name || target.aliases.contains(&dependency.name),
        }
    }

    pub fn binding(&self) -> Binding {
        Binding {
            alias: self.alias(),
            requirement: self.req.clone(),
            kind: self.kind().to_string(),
            target: self.target.clone(),
            source: self.source.clone(),
        }
    }

    pub fn edge(&self, from: &Package, to: String, resolved: bool) -> Edge {
        Edge {
            from: from.id.clone(),
            to,
            alias: self.alias(),
            requirement: self.req.clone(),
            source: self.source.clone(),
            kind: self.kind().to_string(),
            target: self.target.clone(),
            optional: self.optional,
            features: self.features.clone(),
            default: self.default,
            resolved,
            propagated: true,
            seat: from.manifest.clone(),
        }
    }
}
