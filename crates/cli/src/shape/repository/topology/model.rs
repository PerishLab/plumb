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
    pub candidates: Vec<super::registry::Candidate>,
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
    pub versions: Vec<String>,
    pub resolved: bool,
    pub propagated: bool,
    pub seat: PathBuf,
}

#[derive(Clone, Serialize)]
pub(crate) struct Refusal {
    pub reason: String,
    pub path: Vec<Edge>,
}

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd, Serialize)]
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
            versions: Vec::new(),
            resolved,
            propagated: true,
            seat: from.manifest.clone(),
        }
    }
}

#[cfg(test)]
mod groups {
    use super::super::tests::{DOMAIN, FOREIGN, Fixture};
    use super::super::{Report, declared};

    const GROUP: &str = "[dependency]\nroot=true\n[dependency.groups]\nlock=['lock','macros']\n";

    fn fixture() -> Fixture {
        let mut fixture = Fixture::new();
        fixture.package("runtime", "lock", None);
        fixture.package("derive", "macros", None);
        fixture.packages[1]["targets"][0]["kind"] = serde_json::json!(["proc-macro"]);
        fixture.edge("runtime", "derive", None);
        fixture
    }

    fn report(fixture: &Fixture, text: &str) -> Report {
        let declaration = declared(text.as_bytes()).unwrap();
        fixture.named(declaration.root, declaration.groups)
    }

    #[test]
    fn internal() {
        let fixture = fixture();
        assert!(!fixture.report(true).refusals.is_empty());
        let report = report(&fixture, GROUP);
        assert!(report.refusals.is_empty());
        assert!(report.unknown.is_empty());
        assert_eq!(report.edges.len(), 1);
        assert_eq!(report.groups["lock"], ["lock", "macros"]);
    }

    #[test]
    fn crossed() {
        let mut fixture = fixture();
        fixture.package("other", "token", None);
        fixture.edge("derive", "other", Some("build"));
        let text = format!("{GROUP}token=['token']\n");
        let report = report(&fixture, &text);
        assert!(report.unknown.is_empty());
        assert!(
            report
                .refusals
                .iter()
                .any(|refusal| refusal.path.len() == 2)
        );
    }

    #[test]
    fn bridge() {
        let mut fixture = fixture();
        fixture.package("bridge", "bridge", Some(FOREIGN));
        fixture.edge("runtime", "bridge", None);
        fixture.edge("bridge", "derive", Some("build"));
        let report = report(&fixture, GROUP);
        assert!(report.unknown.is_empty());
        assert!(
            report
                .refusals
                .iter()
                .any(|refusal| refusal.path.len() == 2)
        );
    }

    #[test]
    fn domain() {
        let mut fixture = fixture();
        fixture.package("published", "macros", Some(DOMAIN));
        fixture.edge("runtime", "published", None);
        assert!(!report(&fixture, GROUP).refusals.is_empty());
    }

    #[test]
    fn cycle() {
        let mut fixture = fixture();
        fixture.edge("derive", "runtime", Some("dev"));
        let report = report(&fixture, GROUP);
        assert_eq!(report.cycles.len(), 1);
        assert!(
            report
                .refusals
                .iter()
                .any(|refusal| refusal.reason.contains("cycle"))
        );
    }

    #[test]
    fn invalid() {
        let fixture = fixture();
        for members in [
            "lock=['lock']",
            "lock=[]",
            "lock=['lock','unknown']",
            "lock=['lock','lock','macros']",
            "lock=['lock','macros']\nother=['macros']",
            "''=['lock','macros']",
        ] {
            let text = format!("[dependency]\nroot=true\n[dependency.groups]\n{members}\n");
            let report = report(&fixture, &text);
            assert!(!report.unknown.is_empty(), "{members}");
            assert!(
                !report.refusals.is_empty(),
                "invalid groups grant no internal exemptions"
            );
        }
    }
}
