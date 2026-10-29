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

struct Material {
    bytes: BTreeMap<String, Vec<u8>>,
    hashes: BTreeMap<String, String>,
    prepared: BTreeSet<String>,
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
    fn confirm(&self) -> Result<(), String> {
        Ok(())
    }
    fn indices(&self) -> &BTreeMap<String, String> {
        &self.hashes
    }
}

fn material() -> Material {
    let mut material = Material {
        bytes: BTreeMap::new(),
        hashes: BTreeMap::new(),
        prepared: BTreeSet::new(),
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

fn report(material: &mut Material, checksum: &str) -> Report {
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

#[test]
fn complete() {
    let mut material = material();
    let report = report(&mut material, &"0".repeat(64));
    assert!(material.prepared.contains("hidden"));
    assert!(report.closure.as_ref().unwrap().complete);
    assert!(report.unknown.is_empty(), "{:?}", report.unknown);
    assert!(report.refusals.is_empty());
    let hidden = report
        .nodes
        .iter()
        .find(|node| node.name == "hidden" && !node.candidates.is_empty())
        .unwrap();
    assert!(hidden.version.is_none());
    assert_eq!(hidden.candidates.len(), 2);
    assert!(
        report
            .edges
            .iter()
            .any(|edge| edge.optional && !edge.resolved && edge.to == hidden.id)
    );
}

#[test]
fn hidden() {
    let mut material = material();
    material.bytes.insert(
        "hidden".into(),
        format!(
            "{}\n{}\n",
            entry("hidden", "1.0.0", vec![]),
            entry(
                "hidden",
                "1.1.0",
                vec![dependency(
                    "product",
                    Some("sparse+https://cargo.perish.uk/")
                )]
            )
        )
        .into_bytes(),
    );
    let report = report(&mut material, &"0".repeat(64));
    assert!(!report.refusals.is_empty());
    assert!(!report.unknown.is_empty());
    assert!(
        report
            .refusals
            .iter()
            .any(|refusal| refusal.path.iter().any(|edge| edge.to.contains("product")))
    );
}

#[test]
fn missing() {
    let mut material = material();
    material.bytes.remove("hidden");
    let report = report(&mut material, &"0".repeat(64));
    assert!(!report.closure.as_ref().unwrap().complete);
    assert!(
        report
            .unknown
            .iter()
            .any(|message| message.contains("unbound declared dependency"))
    );
}

#[test]
fn checksum() {
    let report = report(&mut material(), &"1".repeat(64));
    assert!(!report.closure.as_ref().unwrap().complete);
    assert!(
        report
            .unknown
            .iter()
            .any(|message| message.contains("checksum"))
    );
}

#[test]
fn schema() {
    let mut material = material();
    let mut record = entry("hidden", "1.0.0", vec![]);
    record["v"] = json!(3);
    material
        .bytes
        .insert("hidden".into(), serde_json::to_vec(&record).unwrap());
    let report = report(&mut material, &"0".repeat(64));
    assert!(!report.closure.as_ref().unwrap().complete);
    assert!(
        report
            .unknown
            .iter()
            .any(|message| message.contains("schema"))
    );
}

#[test]
fn withdrawn() {
    let mut material = material();
    let mut record = entry("hidden", "1.0.0", vec![]);
    record["yanked"] = json!(true);
    material
        .bytes
        .insert("hidden".into(), serde_json::to_vec(&record).unwrap());
    let report = report(&mut material, &"0".repeat(64));
    assert!(report.closure.as_ref().unwrap().complete);
    assert!(report.unknown.is_empty());
    assert!(
        report
            .nodes
            .iter()
            .flat_map(|node| &node.candidates)
            .any(|candidate| candidate.yanked)
    );
}

#[test]
fn truncated() {
    let mut material = material();
    material
        .bytes
        .get_mut("hidden")
        .unwrap()
        .extend_from_slice(b"{\"name\":");
    let report = report(&mut material, &"0".repeat(64));
    assert!(!report.closure.as_ref().unwrap().complete);
    assert!(
        report
            .unknown
            .iter()
            .any(|message| message.contains("invalid index"))
    );
}

#[test]
fn identity() {
    let mut material = material();
    material.bytes.insert(
        "hidden".into(),
        serde_json::to_vec(&entry("different", "1.0.0", vec![])).unwrap(),
    );
    let report = report(&mut material, &"0".repeat(64));
    assert!(!report.closure.as_ref().unwrap().complete);
    assert!(
        report
            .unknown
            .iter()
            .any(|message| message.contains("identity"))
    );
}

#[test]
fn patched() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("Cargo.toml"),
        "[patch.crates-io]\nbridge={path='elsewhere'}\n",
    )
    .unwrap();
    let result = super::super::registry::unmodified(root.path());
    assert!(result.unwrap_err().contains("cannot qualify"));
}

#[test]
fn redirected() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("Cargo.toml"),
        "[package]\nname='specimen'\nversion='0.0.0'\n",
    )
    .unwrap();
    std::fs::create_dir(root.path().join(".cargo")).unwrap();
    std::fs::write(
        root.path().join(".cargo/config.toml"),
        "[registries.crates-io]\nindex='https://example.invalid/index'\n",
    )
    .unwrap();
    let result = super::super::registry::unmodified(root.path());
    assert!(result.unwrap_err().contains("registry override"));
}

#[test]
fn empty() {
    let mut material = material();
    material.bytes.insert(
        "hidden".into(),
        serde_json::to_vec(&entry("hidden", "2.0.0", vec![])).unwrap(),
    );
    let report = report(&mut material, &"0".repeat(64));
    assert!(report.closure.as_ref().unwrap().complete);
    assert!(report.unknown.is_empty());
}

#[test]
fn absent() {
    let mut material = material();
    material.bytes.insert(
        "bridge".into(),
        serde_json::to_vec(&entry("bridge", "2.0.0", vec![])).unwrap(),
    );
    let report = report(&mut material, &"0".repeat(64));
    assert!(!report.closure.as_ref().unwrap().complete);
    assert!(
        report
            .unknown
            .iter()
            .any(|message| message.contains("omits locked"))
    );
}
