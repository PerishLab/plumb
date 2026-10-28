use super::Report;
use super::model::{Declared, Dependency, Edge, Metadata, Node, Package, Resolve};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Graph {
    pub nodes: BTreeMap<String, Node>,
    pub edges: Vec<Edge>,
    pub roots: BTreeSet<String>,
    pub unknown: Vec<String>,
    pub binaries: Vec<String>,
}

impl Graph {
    pub fn parse(bytes: &[u8], source: Option<&str>) -> Result<Self, String> {
        let metadata: Metadata = serde_json::from_slice(bytes)
            .map_err(|error| format!("cannot decode topology metadata: {error}"))?;
        if metadata.packages.len() > 20000 {
            return Err("topology exceeds 20000 package budget".into());
        }
        let resolve = metadata
            .resolve
            .as_ref()
            .ok_or("topology has no resolve graph")?;
        let workspace = metadata.workspace.iter().cloned().collect::<BTreeSet<_>>();
        let mut graph = Self {
            nodes: BTreeMap::new(),
            edges: Vec::new(),
            roots: BTreeSet::new(),
            unknown: Vec::new(),
            binaries: Vec::new(),
        };
        let owned = source.is_some_and(|value| value.starts_with("PerishLab/"));
        for package in &metadata.packages {
            let local = workspace.contains(&package.id);
            let features = resolve.nodes.iter().find(|node| node.id == package.id);
            let node = Node {
                id: package.id.clone(),
                name: package.name.clone(),
                version: Some(package.version.clone()),
                source: package.source.clone(),
                domain: local && owned || domain(package.source.as_deref()),
                workspace: local,
                manifest: Some(package.manifest.clone()),
                features: features
                    .map(|node| node.features.clone())
                    .unwrap_or_default(),
                aliases: package
                    .targets
                    .iter()
                    .filter_map(|target| target.name.clone())
                    .collect(),
            };
            if graph.nodes.insert(package.id.clone(), node).is_some() {
                return Err("topology repeats a package identity".into());
            }
            if local {
                graph.member(package);
            }
            if package.source.is_none() && !local {
                graph.unknown.push(format!(
                    "path package {} is outside the qualified workspace",
                    package.id
                ));
            }
            if package
                .source
                .as_deref()
                .is_some_and(|source| source.starts_with("git+"))
            {
                graph.unknown.push(format!(
                    "Git package authority is not qualified: {}",
                    package.id
                ));
            }
        }
        for package in &metadata.packages {
            graph.resolve(package, resolve)?;
            graph.declarations(package, &metadata.packages);
            if graph.edges.len() > 200000 {
                return Err("topology exceeds 200000 edge budget".into());
            }
        }
        Ok(graph)
    }

    fn resolve(&mut self, package: &Package, resolve: &Resolve) -> Result<(), String> {
        let Some(resolved) = resolve.nodes.iter().find(|node| node.id == package.id) else {
            self.unknown
                .push(format!("missing resolve node for {}", package.id));
            return Ok(());
        };
        for dependency in &resolved.deps {
            self.bind(package, dependency)?;
        }
        Ok(())
    }

    fn bind(&mut self, package: &Package, dependency: &Dependency) -> Result<(), String> {
        let target = self
            .nodes
            .get(&dependency.pkg)
            .ok_or_else(|| format!("resolved dependency {} has no package", dependency.pkg))?;
        for condition in &dependency.kinds {
            let declarations = package
                .dependencies
                .iter()
                .filter(|declared| declared.agrees(dependency, condition, target))
                .collect::<Vec<_>>();
            match declarations.as_slice() {
                [declared] => self
                    .edges
                    .push(declared.edge(package, dependency.pkg.clone(), true)),
                _ => self.unknown.push(format!(
                    "cannot bind dependency {} of {}",
                    dependency.name, package.id
                )),
            }
        }
        Ok(())
    }

    fn member(&mut self, package: &Package) {
        for target in &package.targets {
            if target.kind.iter().any(|kind| kind == "bin") {
                self.binaries.push(package.id.clone());
            }
            if target.kind.iter().any(|kind| {
                ["lib", "rlib", "dylib", "cdylib", "staticlib", "proc-macro"]
                    .contains(&kind.as_str())
            }) {
                self.roots.insert(package.id.clone());
            }
        }
    }

    fn declarations(&mut self, package: &Package, packages: &[Package]) {
        let seen = self
            .edges
            .iter()
            .filter(|edge| edge.from == package.id)
            .map(Edge::binding)
            .collect::<BTreeSet<_>>();
        for declaration in &package.dependencies {
            if !seen.contains(&declaration.binding()) {
                self.missing(package, declaration, packages);
            }
        }
    }

    fn missing(&mut self, package: &Package, declared: &Declared, packages: &[Package]) {
        let matches = packages
            .iter()
            .filter(|target| matches(declared, target))
            .collect::<Vec<_>>();
        let target = match matches.as_slice() {
            [target] => target.id.clone(),
            _ => {
                let id = format!(
                    "declared:{}:{}:{:?}",
                    declared.name, declared.req, declared.source
                );
                self.nodes.entry(id.clone()).or_insert_with(|| Node {
                    id: id.clone(),
                    name: declared.name.clone(),
                    version: None,
                    source: declared.source.clone(),
                    domain: domain(declared.source.as_deref()),
                    workspace: false,
                    manifest: None,
                    features: Vec::new(),
                    aliases: Vec::new(),
                });
                self.unbound(package, declared);
                id
            }
        };
        self.edges.push(declared.edge(package, target, false));
    }

    fn unbound(&mut self, package: &Package, declared: &Declared) {
        let propagated = declared.kind() != "dev" || self.nodes[&package.id].workspace;
        if propagated {
            self.unknown.push(format!(
                "unbound declared dependency {} {} of {} ({}, {:?})",
                declared.name,
                declared.req,
                package.id,
                declared.kind(),
                declared.target
            ));
        }
    }

    pub fn fill(mut self, report: &mut Report) {
        for edge in &mut self.edges {
            edge.propagated = edge.kind != "dev" || self.nodes[&edge.from].workspace;
        }
        report.unknown.append(&mut self.unknown);
        report.cycles = self.cycles(report);
        if report.root == Some(true) {
            report.refusals = self.refusals(&mut report.unknown);
            if self.roots.is_empty() {
                report
                    .unknown
                    .push("root workspace has no library member".into());
            }
        }
        report.nodes = self.nodes.into_values().collect();
        report.edges = self.edges;
    }
}

fn domain(source: Option<&str>) -> bool {
    source == Some(crate::catalog::set::CARGO.index)
}

fn matches(declared: &Declared, package: &Package) -> bool {
    if declared.name != package.name || declared.source != package.source {
        return false;
    }
    if let Some(path) = &declared.path {
        return path.canonicalize().ok().is_some_and(|path| {
            package
                .manifest
                .parent()
                .and_then(|path| path.canonicalize().ok())
                == Some(path)
        });
    }
    match (
        semver::VersionReq::parse(&declared.req),
        semver::Version::parse(&package.version),
    ) {
        (Ok(requirement), Ok(version)) => requirement.matches(&version),
        _ => false,
    }
}
