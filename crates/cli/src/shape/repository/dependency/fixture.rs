use super::super::{
    Report,
    evidence::{Source, digest},
    model::Metadata,
    read::Graph,
    registry::SOURCE,
    tests::{Fixture, dependency, entry},
};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Material {
    pub(super) bytes: BTreeMap<String, Vec<u8>>,
    pub(super) hashes: BTreeMap<String, String>,
    pub(super) prepared: BTreeSet<String>,
    pub(super) fresh: BTreeMap<String, Result<Vec<u8>, String>>,
    pub(super) calls: usize,
}

impl Source for Material {
    fn prepare(&mut self, names: &BTreeSet<String>) {
        self.prepared.extend(names.iter().cloned());
    }
    fn read(&mut self, name: &str) -> Result<Vec<u8>, String> {
        let bytes = self
            .bytes
            .get(name)
            .ok_or_else(|| format!("missing index {name}"))?
            .clone();
        self.hashes.insert(name.into(), digest(&bytes));
        Ok(bytes)
    }
    fn refresh(&mut self, name: &str) -> Result<Option<Vec<u8>>, String> {
        self.calls += 1;
        let bytes = self.fresh.remove(name).transpose()?;
        if let Some(bytes) = &bytes {
            self.hashes.insert(name.into(), digest(bytes));
        }
        Ok(bytes)
    }
    fn confirm(&self) -> Result<(), String> {
        Ok(())
    }
    fn indices(&self) -> &BTreeMap<String, String> {
        &self.hashes
    }
}

pub(super) fn material() -> Material {
    let mut material = Material {
        bytes: BTreeMap::new(),
        hashes: BTreeMap::new(),
        prepared: BTreeSet::new(),
        fresh: BTreeMap::new(),
        calls: 0,
    };
    let bridge = entry("bridge", "1.0.0", vec![dependency("hidden", None)]);
    material
        .bytes
        .insert("bridge".into(), serde_json::to_vec(&bridge).unwrap());
    material.bytes.insert(
        "hidden".into(),
        format!(
            "{}\n{}\n",
            entry("hidden", "1.0.0", vec![]),
            entry("hidden", "1.1.0", vec![])
        )
        .into_bytes(),
    );
    material
}

fn fixture() -> Vec<u8> {
    let mut fixture = Fixture::new();
    fixture.package("root", "root", None);
    fixture.package("bridge", "bridge", Some(SOURCE));
    fixture.edge("root", "bridge", None);
    fixture.packages[1]["dependencies"].as_array_mut().unwrap().push(json!({
        "name":"hidden","source":SOURCE,"req":"^1","path":null,"kind":null,"target":"cfg(unix)","rename":null,"optional":true,"features":[],"uses_default_features":true
    }));
    serde_json::to_vec(&json!({"packages":fixture.packages,"workspace_members":fixture.workspace,"resolve":{"nodes":fixture.nodes}})).unwrap()
}

pub(super) fn report(material: &mut Material, checksum: &str) -> Report {
    let bytes = fixture();
    let metadata: Metadata = serde_json::from_slice(&bytes).unwrap();
    let mut graph = Graph::parse(&bytes, Some("PerishLab/specimen")).unwrap();
    let lock = format!(
        "version=4\n[[package]]\nname='bridge'\nversion='1.0.0'\nsource='{SOURCE}'\nchecksum='{checksum}'\n"
    );
    let mut report = Report::empty();
    report.root = Some(true);
    report.closure = Some(graph.supplement(&metadata, lock.as_bytes(), material));
    graph.fill(&mut report);
    report
}

fn stale() -> Material {
    let mut held = material();
    let fresh = held
        .bytes
        .insert(
            "bridge".into(),
            serde_json::to_vec(&entry("bridge", "0.9.0", vec![])).unwrap(),
        )
        .unwrap();
    held.fresh.insert("bridge".into(), Ok(fresh));
    held
}

#[test]
fn missing() {
    let mut held = stale();
    let report = report(&mut held, &"0".repeat(64));
    assert!(report.closure.as_ref().unwrap().complete);
    assert!(report.unknown.is_empty(), "{:?}", report.unknown);
    assert_eq!(held.calls, 1);
}

#[test]
fn failure() {
    let mut held = stale();
    held.fresh
        .insert("bridge".into(), Err("refresh unavailable".into()));
    let report = report(&mut held, &"0".repeat(64));
    assert!(!report.closure.as_ref().unwrap().complete);
    assert!(
        report
            .unknown
            .iter()
            .any(|v| v.contains("refresh unavailable"))
    );
    assert_eq!(held.calls, 1);
}

#[test]
fn checksum() {
    let mut held = stale();
    let report = report(&mut held, &"1".repeat(64));
    assert!(!report.closure.as_ref().unwrap().complete);
    assert!(report.unknown.iter().any(|v| v.contains("checksum")));
    assert_eq!(held.calls, 1);
}

#[test]
fn fresh() {
    let mut held = material();
    let report = report(&mut held, &"0".repeat(64));
    assert!(report.closure.as_ref().unwrap().complete);
    assert_eq!(held.calls, 0);
}
