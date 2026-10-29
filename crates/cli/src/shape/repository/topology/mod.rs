mod closure;
mod evidence;
#[path = "graph.rs"]
mod graph;
#[path = "model.rs"]
mod model;
#[path = "probe.rs"]
mod probe;
#[path = "read.rs"]
mod read;
mod registry;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use model::{Edge, Node, Refusal};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, Serialize)]
pub(crate) struct Report {
    pub schema: &'static str,
    pub root: Option<bool>,
    pub groups: BTreeMap<String, Vec<String>>,
    pub context: &'static str,
    pub input: String,
    pub source: Option<String>,
    pub tree: Option<String>,
    pub tool: Option<String>,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub cycles: Vec<Vec<Edge>>,
    pub refusals: Vec<Refusal>,
    pub unknown: Vec<String>,
    pub tests: Vec<Edge>,
    pub closure: Option<evidence::Evidence>,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Declaration {
    root: bool,
    #[serde(default)]
    groups: BTreeMap<String, Vec<String>>,
}

pub(crate) fn world(root: &Path, manifest: Option<&str>) -> Result<Option<String>, String> {
    if !declared(manifest.unwrap_or_default().as_bytes())?.root {
        return Ok(None);
    }
    if root.join("Cargo.toml").exists() && !root.join("package.json").exists() {
        crate::diagnostic::measure("topology.prepare", || plumb::packages::cargo::fetch(root))?;
    }
    let report = Report::read(root);
    if !report.unknown.is_empty() || !report.refusals.is_empty() {
        return Err(format!(
            "strict root topology refuses reuse: {} unknown, {} violations; run plumb doctor --json",
            report.unknown.len(),
            report.refusals.len()
        ));
    }
    Ok(Some(report.input))
}

pub(crate) fn bind(name: &str, input: String, topology: Option<&str>) -> String {
    match (name, topology) {
        ("guard/plumb", Some(topology)) => fingerprint([input.as_bytes(), topology.as_bytes()]),
        _ => input,
    }
}

impl Report {
    pub fn read(root: &Path) -> Self {
        let mut report = Self::empty();
        let declaration = match std::fs::read(root.join("plumb.toml")) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(error) => {
                report.root = None;
                report
                    .unknown
                    .push(format!("cannot read root declaration: {error}"));
                return report;
            }
        };
        report.root = match declared(&declaration) {
            Ok(value) => {
                report.groups = value.groups;
                Some(value.root)
            }
            Err(error) => {
                report.unknown.push(error);
                None
            }
        };
        let lock = std::fs::read(root.join("Cargo.lock")).unwrap_or_default();
        if root.join("package.json").exists() && report.root != Some(false) {
            report
                .unknown
                .push("Node root closure is not yet supported".into());
        }
        if !root.join("Cargo.toml").exists() {
            if report.root == Some(true) {
                report
                    .unknown
                    .push("root declaration has no Cargo workspace".into());
            }
            return report;
        }
        let probe = probe::Probe { root };
        match probe.source() {
            Ok(source) => report.source = Some(source),
            Err(error) => report.unknown.push(error),
        }
        match probe.tree() {
            Ok(tree) => report.tree = Some(tree),
            Err(error) => report.unknown.push(error),
        }
        match probe.version() {
            Ok(tool) => report.tool = Some(tool),
            Err(error) => report.unknown.push(error),
        }
        match probe.read() {
            Ok(bytes) => {
                let scope = format!("{:?}{:?}{:?}", report.source, report.tree, report.tool);
                report.input =
                    fingerprint([declaration.as_slice(), &lock, &bytes, scope.as_bytes()]);
                if let Err(error) = report.capture(root, &bytes, &lock) {
                    report.unknown.push(error);
                }
                report.confirm(&probe, [&declaration, &lock, &bytes]);
            }
            Err(error) => report.unknown.push(error),
        }
        if let Some(closure) = &report.closure {
            report.context = "frozen-all-features+declaration-candidates";
            report.input = fingerprint([report.input.as_bytes(), closure.input.as_bytes()]);
            if let Err(error) = registry::unmodified(root) {
                report.unknown.push(error);
            }
        }
        report.unknown.sort();
        report.unknown.dedup();
        report
    }

    fn capture(&mut self, root: &Path, bytes: &[u8], lock: &[u8]) -> Result<(), String> {
        let mut graph = read::Graph::parse(bytes, self.source.as_deref())?;
        if self.root == Some(true)
            && graph
                .edges
                .iter()
                .any(|edge| !edge.resolved && edge.source.as_deref() == Some(registry::SOURCE))
        {
            match registry::unmodified(root).and_then(|()| evidence::Reader::new()) {
                Ok(mut reader) => {
                    let metadata =
                        serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
                    self.closure = Some(graph.supplement(&metadata, lock, &mut reader));
                }
                Err(error) => graph.unknown.push(error),
            }
        }
        graph.fill(self);
        Ok(())
    }

    fn confirm(&mut self, probe: &probe::Probe<'_>, inputs: [&[u8]; 3]) {
        for (name, before) in ["plumb.toml", "Cargo.lock"].into_iter().zip(inputs) {
            if std::fs::read(probe.root.join(name)).unwrap_or_default() != before {
                self.unknown
                    .push(format!("topology input changed during capture: {name}"));
            }
        }
        match probe.read() {
            Ok(bytes) if bytes == inputs[2] => {}
            Ok(_) => self
                .unknown
                .push("topology metadata changed during capture".into()),
            Err(error) => self.unknown.push(error),
        }
        let before = (self.source.clone(), self.tree.clone(), self.tool.clone());
        let after = (probe.source().ok(), probe.tree().ok(), probe.version().ok());
        if before != after {
            self.unknown
                .push("topology source, tree or tool changed during capture".into());
        }
    }

    pub fn empty() -> Self {
        Self {
            schema: "plumb.topology/v3",
            root: Some(false),
            groups: BTreeMap::new(),
            context: "frozen-all-features",
            input: String::new(),
            source: None,
            tree: None,
            tool: None,
            nodes: Vec::new(),
            edges: Vec::new(),
            cycles: Vec::new(),
            refusals: Vec::new(),
            unknown: Vec::new(),
            tests: Vec::new(),
            closure: None,
        }
    }
}

fn declared(bytes: &[u8]) -> Result<Declaration, String> {
    if bytes.is_empty() {
        return Ok(Declaration::default());
    }
    let text = std::str::from_utf8(bytes).map_err(|error| error.to_string())?;
    let document: toml::Table = toml::from_str(text).map_err(|error| error.to_string())?;
    let Some(value) = document.get("dependency") else {
        return Ok(Declaration::default());
    };
    let declaration: Declaration = value
        .clone()
        .try_into()
        .map_err(|error| format!("invalid dependency declaration: {error}"))?;
    if !declaration.root {
        return Err("dependency.root must be true; false is not a root exemption".into());
    }
    Ok(declaration)
}

impl read::Graph {
    fn group(&mut self, report: &Report) -> Result<(), String> {
        if report.groups.is_empty() {
            return Ok(());
        }
        let mut assigned = BTreeMap::new();
        for (group, members) in &report.groups {
            if group.trim().is_empty() || members.is_empty() {
                return Err("root capability groups must have a name and actual members".into());
            }
            for name in members {
                let id = self.identify(name)?;
                if assigned.insert(id, group.clone()).is_some() {
                    return Err(format!(
                        "root capability member {name} occurs more than once"
                    ));
                }
            }
        }
        if assigned.len() != self.roots.len() {
            return Err("root capability groups must assign every workspace library".into());
        }
        self.groups = assigned;
        Ok(())
    }

    fn identify(&self, name: &str) -> Result<String, String> {
        let matches = self
            .nodes
            .values()
            .filter(|node| node.name == name && self.roots.contains(&node.id))
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [node] => Ok(node.id.clone()),
            _ => Err(format!(
                "root capability member {name} is not one exact workspace library"
            )),
        }
    }

    fn internal(&self, edge: &Edge) -> bool {
        self.groups
            .get(&edge.from)
            .is_some_and(|group| self.groups.get(&edge.to) == Some(group))
    }
}

fn fingerprint<const N: usize>(parts: [&[u8]; N]) -> String {
    let mut hash = Sha256::new();
    for part in parts {
        hash.update(part.len().to_le_bytes());
        hash.update(part);
    }
    format!("{:x}", hash.finalize())
}
