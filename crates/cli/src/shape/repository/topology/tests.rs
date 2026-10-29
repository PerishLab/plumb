use super::{Report, declared, read::Graph};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub(super) struct Fixture {
    pub(super) packages: Vec<Value>,
    pub(super) nodes: Vec<Value>,
    pub(super) workspace: Vec<String>,
}

impl Fixture {
    pub(super) fn new() -> Self {
        Self {
            packages: Vec::new(),
            nodes: Vec::new(),
            workspace: Vec::new(),
        }
    }

    pub(super) fn package(&mut self, id: &str, name: &str, source: Option<&str>) {
        self.packages.push(json!({
            "id":id,"name":name,"version":"1.0.0","source":source,
            "manifest_path":format!("/fixture/{id}/Cargo.toml"),
            "targets":[{"kind":["lib"]}],"dependencies":[]
        }));
        self.nodes.push(json!({"id":id,"features":[],"deps":[]}));
        if source.is_none() {
            self.workspace.push(id.to_string());
        }
    }

    pub(super) fn edge(&mut self, from: &str, to: &str, kind: Option<&str>) {
        let target = self
            .packages
            .iter()
            .find(|p| p["id"] == to)
            .unwrap()
            .clone();
        let package = self.packages.iter_mut().find(|p| p["id"] == from).unwrap();
        package["dependencies"].as_array_mut().unwrap().push(json!({
            "name":target["name"],"source":target["source"],"req":"=1.0.0",
            "path":null,"kind":kind,"target":null,"rename":null,
            "optional":false,"features":[],"uses_default_features":true
        }));
        self.nodes.iter_mut().find(|p| p["id"] == from).unwrap()["deps"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "name":target["name"].as_str().unwrap().replace('-',"_"),
                "pkg":to,"dep_kinds":[{"kind":kind,"target":null}]
            }));
    }

    pub(super) fn report(&self, root: bool) -> Report {
        self.named(root, BTreeMap::new())
    }

    pub(super) fn named(&self, root: bool, groups: BTreeMap<String, Vec<String>>) -> Report {
        let bytes = serde_json::to_vec(&json!({
            "packages":self.packages,"workspace_members":self.workspace,
            "resolve":{"nodes":self.nodes}
        }))
        .unwrap();
        let graph = Graph::parse(&bytes, Some("PerishLab/specimen")).unwrap();
        let mut report = Report::empty();
        report.root = Some(root);
        report.groups = groups;
        graph.fill(&mut report);
        report
    }
}

pub(super) const DOMAIN: &str = "sparse+https://cargo.perish.uk/";
pub(super) const FOREIGN: &str = "registry+https://github.com/rust-lang/crates.io-index";

#[test]
fn technical() {
    for kind in [None, Some("dev"), Some("build")] {
        let mut fixture = Fixture::new();
        fixture.package("runtime", "runtime", None);
        fixture.package("macros", "macros", None);
        fixture.edge("runtime", "macros", kind);
        fixture.packages[0]["dependencies"][0]["optional"] = json!(true);
        fixture.packages[0]["dependencies"][0]["target"] = json!("cfg(windows)");
        fixture.nodes[0]["deps"][0]["dep_kinds"][0]["target"] = json!("cfg(windows)");
        let groups = BTreeMap::from([("lock".into(), vec!["runtime".into(), "macros".into()])]);
        let report = fixture.named(true, groups);
        assert!(report.unknown.is_empty());
        assert!(report.refusals.is_empty());
        assert_eq!(report.edges[0].target.as_deref(), Some("cfg(windows)"));
        assert!(report.edges[0].optional);
    }
}

#[test]
fn orthogonal() {
    let mut fixture = Fixture::new();
    fixture.package("lock", "radicula-lock", None);
    fixture.package("token", "radicula-token", None);
    let report = fixture.report(true);
    assert!(report.refusals.is_empty());
    assert!(report.unknown.is_empty());
    fixture.edge("lock", "token", None);
    let report = fixture.report(true);
    assert_eq!(report.refusals.len(), 1);
    assert_eq!(report.refusals[0].path[0].to, "token");
}

#[test]
fn versions() {
    let mut fixture = Fixture::new();
    fixture.package("plumb2", "plumb", None);
    fixture.package("locus1", "locus", Some(DOMAIN));
    fixture.package("plumb1", "plumb", Some(DOMAIN));
    fixture.edge("plumb2", "locus1", None);
    fixture.edge("locus1", "plumb1", None);
    let report = fixture.report(false);
    assert_eq!(report.cycles.len(), 1);
    assert_eq!(report.cycles[0].len(), 2);
    assert!(report.tests.is_empty());
    fixture.packages[2]["source"] = json!(FOREIGN);
    fixture.packages[1]["dependencies"][0]["source"] = json!(FOREIGN);
    assert!(fixture.report(false).cycles.is_empty());
}

#[test]
fn bridge() {
    let mut fixture = Fixture::new();
    fixture.package("root", "radicula-lock", None);
    fixture.package("bridge", "bridge", Some(FOREIGN));
    fixture.package("plumb", "plumb", Some(DOMAIN));
    fixture.edge("root", "bridge", None);
    fixture.edge("bridge", "plumb", Some("build"));
    let report = fixture.report(true);
    assert_eq!(report.refusals[0].path.len(), 2);
    assert_eq!(report.refusals[0].path[1].kind, "build");
    fixture.packages[2]["source"] = json!(FOREIGN);
    fixture.packages[1]["dependencies"][0]["source"] = json!(FOREIGN);
    assert!(fixture.report(true).refusals.is_empty());
}

#[test]
fn branches() {
    for kind in [None, Some("dev"), Some("build")] {
        let mut fixture = Fixture::new();
        fixture.package("root", "radicula-lock", None);
        fixture.package("target", "plumb", Some(DOMAIN));
        fixture.edge("root", "target", kind);
        fixture.packages[0]["dependencies"][0]["optional"] = json!(true);
        fixture.packages[0]["dependencies"][0]["target"] = json!("cfg(windows)");
        fixture.nodes[0]["deps"][0]["dep_kinds"][0]["target"] = json!("cfg(windows)");
        let report = fixture.report(true);
        assert_eq!(report.refusals.len(), 1);
        assert_eq!(
            report.refusals[0].path[0].target.as_deref(),
            Some("cfg(windows)")
        );
        assert!(report.refusals[0].path[0].optional);
        fixture.packages.pop();
        fixture.nodes.pop();
        fixture.nodes[0]["deps"] = json!([]);
        let partial = fixture.report(true);
        assert!(!partial.unknown.is_empty());
        assert!(
            !partial.refusals.is_empty(),
            "known domain edge survives an incomplete closure"
        );
    }
}

#[test]
fn selftest() {
    let mut fixture = Fixture::new();
    fixture.package("self", "plumb", None);
    fixture.edge("self", "self", Some("dev"));
    let ordinary = fixture.report(false);
    assert!(ordinary.cycles.is_empty());
    assert_eq!(ordinary.tests.len(), 1);
    let root = fixture.report(true);
    assert_eq!(root.refusals.len(), 1);
    assert_eq!(root.cycles.len(), 1);
    assert!(root.tests.is_empty());
}

#[test]
fn aliases() {
    let mut fixture = Fixture::new();
    fixture.package("root", "root", None);
    fixture.package("foreign", "shared", Some(FOREIGN));
    fixture.edge("root", "foreign", None);
    let mut latent = fixture.packages[0]["dependencies"][0].clone();
    latent["rename"] = json!("domain");
    latent["source"] = json!(DOMAIN);
    fixture.packages[0]["dependencies"]
        .as_array_mut()
        .unwrap()
        .push(latent);
    let report = fixture.report(true);
    assert_eq!(report.edges.len(), 2);
    assert_eq!(report.refusals.len(), 1);
    assert!(!report.unknown.is_empty());
}

#[test]
fn declaration() {
    assert!(!declared(b"[identity]\nns='specimen'\n").unwrap().root);
    assert!(declared(b"[dependency]\nroot=true\n").unwrap().root);
    for text in [
        "[dependency]\nroot=false\n",
        "[dependency]\nroot='true'\n",
        "[dependency]\nroot=true\nallow=['plumb']\n",
        "[dependency]\n",
    ] {
        assert!(declared(text.as_bytes()).is_err(), "{text}");
    }
}

#[test]
fn development() {
    let mut fixture = Fixture::new();
    fixture.package("root", "radicula-lock", None);
    fixture.package("foreign", "foreign", Some(FOREIGN));
    fixture.package("plumb", "plumb", Some(DOMAIN));
    fixture.edge("root", "foreign", Some("dev"));
    fixture.edge("foreign", "plumb", Some("dev"));
    fixture.packages.pop();
    fixture.nodes.pop();
    fixture.nodes[1]["deps"] = json!([]);
    let report = fixture.report(true);
    assert!(report.refusals.is_empty());
    assert!(report.unknown.is_empty());
    assert!(
        report
            .edges
            .iter()
            .any(|edge| edge.kind == "dev" && !edge.propagated)
    );
    fixture.packages[1]["dependencies"][0]["kind"] = Value::Null;
    let required = fixture.report(true);
    assert!(!required.refusals.is_empty());
    assert!(!required.unknown.is_empty());
}

#[test]
fn targets() {
    let mut fixture = Fixture::new();
    fixture.package("macro", "root-macro", None);
    fixture.package("peer", "peer", None);
    fixture.packages[0]["targets"][0]["kind"] = json!(["proc-macro"]);
    fixture.edge("macro", "peer", Some("build"));
    let report = fixture.report(true);
    assert_eq!(report.refusals.len(), 1);
    assert!(report.unknown.is_empty());
    fixture.packages[0]["targets"][0]["kind"] = json!(["bin"]);
    assert!(
        fixture
            .report(true)
            .refusals
            .iter()
            .any(|refusal| refusal.reason.contains("binary"))
    );
}

pub(super) fn dependency(name: &str, registry: Option<&str>) -> serde_json::Value {
    json!({"name":name,"req":"^1","features":[],"optional":true,"default_features":true,"target":"cfg(unix)","kind":"normal","registry":registry})
}

pub(super) fn entry(name: &str, version: &str, deps: Vec<serde_json::Value>) -> serde_json::Value {
    json!({"name":name,"vers":version,"cksum":"0".repeat(64),"deps":deps,"features":{},"yanked":false})
}

#[test]
fn worlds() {
    let first = super::bind("guard/plumb", "tree".into(), Some("index-a"));
    assert_eq!(first.len(), 64);
    assert_ne!(
        first,
        super::bind("guard/plumb", "tree".into(), Some("index-b"))
    );
    assert_eq!(
        super::bind("guard/rust", "tree".into(), Some("index-a")),
        "tree"
    );
    assert_eq!(
        super::world(std::path::Path::new("absent"), None).unwrap(),
        None
    );
    assert!(
        super::world(
            std::path::Path::new("absent"),
            Some("[dependency]\nroot=false")
        )
        .is_err()
    );
}
