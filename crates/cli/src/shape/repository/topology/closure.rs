#[cfg(test)]
#[path = "declaration.rs"]
mod tests;

use super::{
    evidence::{Evidence, Source, digest},
    model::{Metadata, Package},
    read::Graph,
    registry::{self, Entry},
};
use semver::{Version, VersionReq};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub(super) struct Closure<'a> {
    graph: &'a mut Graph,
    indices: BTreeMap<String, Vec<Entry>>,
    failed: BTreeSet<String>,
    versions: BTreeSet<(String, String)>,
    ranges: BTreeSet<(String, String)>,
    queue: VecDeque<(String, String)>,
    locked: BTreeMap<(String, String), String>,
    edges: usize,
    complete: bool,
    merged: BTreeMap<String, usize>,
}

impl Graph {
    pub fn supplement(
        &mut self,
        metadata: &Metadata,
        lock: &[u8],
        reader: &mut impl Source,
    ) -> Evidence {
        let mut closure = Closure {
            graph: self,
            indices: BTreeMap::new(),
            failed: BTreeSet::new(),
            versions: BTreeSet::new(),
            ranges: BTreeSet::new(),
            queue: VecDeque::new(),
            locked: BTreeMap::new(),
            edges: 0,
            complete: true,
            merged: BTreeMap::new(),
        };
        if let Err(error) = closure.lock(lock) {
            closure.unknown(error);
        }
        for package in &metadata.packages {
            closure.seed(package);
        }
        let mut wave = 0;
        while let Some((name, requirement)) = closure.queue.pop_front() {
            if wave == 0 {
                let names = std::iter::once(&name)
                    .chain(closure.queue.iter().map(|(name, _)| name))
                    .filter(|name| {
                        !closure.indices.contains_key(*name) && !closure.failed.contains(*name)
                    })
                    .cloned()
                    .collect();
                reader.prepare(&names);
                wave = closure.queue.len() + 1;
            }
            wave -= 1;
            if closure.ranges.len() >= 20000
                || closure.versions.len() >= 50000
                || closure.edges >= 500000
            {
                closure.unknown("declaration candidate budget exhausted".into());
                break;
            }
            if !closure.ranges.insert((name.clone(), requirement.clone())) {
                continue;
            }
            if let Err(error) = closure.expand(reader, &name, &requirement) {
                closure.unknown(error);
            }
        }
        if let Err(error) = reader.confirm() {
            closure.unknown(error);
        }
        if closure.complete {
            let covered = closure
                .graph
                .edges
                .iter()
                .filter(|edge| !edge.resolved && edge.source.as_deref() == Some(registry::SOURCE))
                .map(|edge| (edge.from.clone(), edge.binding()))
                .collect::<BTreeSet<_>>();
            closure
                .graph
                .pending
                .retain(|binding, _| !covered.contains(binding));
        }
        let input =
            digest(&serde_json::to_vec(reader.indices()).expect("index identity serialization"));
        Evidence {
            schema: "plumb.declaration-closure/v1",
            input,
            indices: reader.indices().clone(),
            versions: closure.versions.len(),
            edges: closure.edges,
            complete: closure.complete,
        }
    }
}

impl Closure<'_> {
    fn unknown(&mut self, message: String) {
        self.complete = false;
        self.graph.unknown.push(message);
    }

    fn lock(&mut self, bytes: &[u8]) -> Result<(), String> {
        let text = std::str::from_utf8(bytes).map_err(|error| error.to_string())?;
        let table: toml::Table = toml::from_str(text).map_err(|error| error.to_string())?;
        for entry in table
            .get("package")
            .and_then(toml::Value::as_array)
            .into_iter()
            .flatten()
        {
            if entry.get("source").and_then(toml::Value::as_str) != Some(registry::SOURCE) {
                continue;
            }
            let name = entry
                .get("name")
                .and_then(toml::Value::as_str)
                .ok_or("locked package has no name")?;
            let version = entry
                .get("version")
                .and_then(toml::Value::as_str)
                .ok_or("locked package has no version")?;
            let checksum = entry
                .get("checksum")
                .and_then(toml::Value::as_str)
                .ok_or("locked package has no checksum")?;
            if self
                .locked
                .insert((name.into(), version.into()), checksum.into())
                .is_some()
            {
                return Err("repeated locked package identity".into());
            }
        }
        Ok(())
    }

    fn seed(&mut self, package: &Package) {
        if package.source.as_deref() == Some(registry::SOURCE) {
            if !self
                .locked
                .contains_key(&(package.name.clone(), package.version.clone()))
            {
                self.unknown(format!(
                    "metadata package has no locked checksum: {}",
                    package.id
                ));
            }
            self.queue
                .push_back((package.name.clone(), format!("={}", package.version)));
        }
        let unresolved = self
            .graph
            .edges
            .iter()
            .filter(|edge| {
                let propagated = edge.kind != "dev" || self.graph.nodes[&package.id].workspace;
                edge.from == package.id && !edge.resolved && propagated
            })
            .cloned()
            .collect::<Vec<_>>();
        for edge in unresolved {
            if edge.source.as_deref() != Some(registry::SOURCE) {
                continue;
            }
            let name = self.graph.nodes[&edge.to].name.clone();
            self.queue
                .push_back((name.clone(), edge.requirement.clone()));
            let mut declared = edge;
            declared.to = registry::cohort(&name, Some(registry::SOURCE));
            self.graph
                .nodes
                .entry(declared.to.clone())
                .or_insert_with(|| registry::node(&name, Some(registry::SOURCE)));
            for original in &mut self.graph.edges {
                if original.from == declared.from
                    && original.binding() == declared.binding()
                    && !original.resolved
                {
                    original.to = declared.to.clone();
                }
            }
        }
    }

    fn expand(
        &mut self,
        reader: &mut impl Source,
        name: &str,
        requirement: &str,
    ) -> Result<(), String> {
        if self.failed.contains(name) {
            return Err(format!("declaration index remains unavailable: {name}"));
        }
        if !self.indices.contains_key(name) {
            let entries = reader
                .read(name)
                .and_then(|bytes| Entry::read(name, &bytes));
            match entries {
                Ok(entries) => {
                    self.indices.insert(name.into(), entries);
                }
                Err(error) => {
                    self.failed.insert(name.into());
                    return Err(error);
                }
            }
        }
        let requirement = VersionReq::parse(requirement).map_err(|error| error.to_string())?;
        let entries = self.indices[name]
            .iter()
            .filter(|entry| {
                Version::parse(&entry.vers).is_ok_and(|version| requirement.matches(&version))
            })
            .cloned()
            .collect::<Vec<_>>();
        if entries.is_empty()
            && self.locked.keys().any(|(package, version)| {
                package == name
                    && Version::parse(version).is_ok_and(|version| requirement.matches(&version))
            })
        {
            return Err(format!(
                "canonical index omits locked candidates for {name} {requirement}"
            ));
        }
        for entry in entries {
            entry.qualify(&self.locked)?;
            if !self.versions.insert((name.into(), entry.vers.clone())) {
                continue;
            }
            if self.versions.len() >= 50000 || self.edges >= 500000 {
                return Err("declaration candidate budget exhausted".into());
            }
            self.attach(&entry);
        }
        Ok(())
    }

    fn attach(&mut self, entry: &Entry) {
        let id = registry::cohort(&entry.name, Some(registry::SOURCE));
        let node = self
            .graph
            .nodes
            .entry(id.clone())
            .or_insert_with(|| registry::node(&entry.name, Some(registry::SOURCE)));
        node.candidates.push(entry.candidate());
        let actual = self
            .graph
            .nodes
            .values()
            .filter(|node| {
                node.source.as_deref() == Some(registry::SOURCE)
                    && node.name == entry.name
                    && node.version.as_deref() == Some(&entry.vers)
            })
            .map(|node| node.id.clone())
            .collect::<Vec<_>>();
        for dependency in entry
            .deps
            .iter()
            .filter(|dependency| dependency.kind != "dev")
        {
            let declaration = dependency.declared();
            let mut edge = dependency.edge(&id);
            edge.versions.push(entry.vers.clone());
            self.graph.nodes.entry(edge.to.clone()).or_insert_with(|| {
                registry::node(&declaration.name, declaration.source.as_deref())
            });
            for from in &actual {
                let mut edge = edge.clone();
                edge.from = from.clone();
                self.graph.merge(edge, &mut self.merged);
            }
            self.graph.merge(edge, &mut self.merged);
            self.edges += 1;
            if declaration.source.as_deref() == Some(registry::SOURCE) {
                self.queue.push_back((declaration.name, declaration.req));
            } else {
                self.unknown(format!(
                    "declaration source is not qualified: {:?}",
                    declaration.source
                ));
            }
        }
    }
}
