use super::model::Plan;
use serde_json::{Value, json};
use std::path::Path;

struct Fixture {
    root: tempfile::TempDir,
    metadata: Value,
    artifacts: Vec<Value>,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            "Cargo.toml",
            "[package]\nname='sample'\nversion='0.1.0'\n",
        );
        let metadata = json!({
            "workspace_root":root.path(),
            "target_directory":root.path().join("target"),
            "workspace_members":["sample"],
            "workspace_default_members":["sample"],
            "packages":[{"id":"sample","name":"sample","manifest_path":root.path().join("Cargo.toml"),"targets":[]}]
        });
        let mut fixture = Self {
            root,
            metadata,
            artifacts: Vec::new(),
        };
        fixture.add("sample", "lib");
        fixture
    }

    fn add(&mut self, name: &str, kind: &str) {
        let source = match kind {
            "lib" => "src/lib.rs".to_string(),
            "bin" => format!("src/bin/{name}.rs"),
            "test" => format!("tests/{name}.rs"),
            _ => panic!("unexpected fixture kind"),
        };
        write(self.root.path(), &source, "");
        let executable = format!("target/debug/deps/{name}");
        write(self.root.path(), &executable, "");
        let target = json!({"name":name,"kind":[kind],"src_path":self.root.path().join(source),"test":true,"doctest":kind=="lib"});
        self.metadata["packages"][0]["targets"]
            .as_array_mut()
            .unwrap()
            .push(target.clone());
        self.artifacts.push(json!({"reason":"compiler-artifact","package_id":"sample","target":target,"profile":{"test":true},"executable":self.root.path().join(executable),"features":[]}));
    }

    fn plan(&self) -> Result<Plan, String> {
        Plan::read(self.root.path(), &self.metadata.to_string(), &self.stream())
    }

    fn stream(&self) -> String {
        self.artifacts
            .iter()
            .map(Value::to_string)
            .chain([json!({"reason":"build-finished","success":true}).to_string()])
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn write(root: &Path, name: &str, body: &str) {
    let path = root.join(name);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
}

#[test]
fn helpers() {
    let mut fixture = Fixture::new();
    fixture.add("regular", "test");
    fixture.add("helper", "bin");
    fixture.metadata["packages"][0]["targets"][2]["test"] = json!(false);
    fixture.artifacts[2]["target"]["test"] = json!(false);
    fixture.artifacts[2]["profile"]["test"] = json!(false);
    let plan = fixture.plan().unwrap();
    assert_eq!(plan.programs.len(), 2);
    assert!(plan.docs);
    assert!(plan.fallback.is_empty(), "{:?}", plan.fallback);
    assert!(
        plan.programs
            .iter()
            .all(|program| program.target.name != "helper")
    );
}

#[test]
fn custom() {
    let mut fixture = Fixture::new();
    fixture.add("custom", "test");
    write(
        fixture.root.path(),
        "Cargo.toml",
        "[package]\nname='sample'\nversion='0.1.0'\n[[test]]\nname='custom'\nharness=false\n",
    );
    let plan = fixture.plan().unwrap();
    assert_eq!(plan.programs.len(), 2);
    assert!(plan.programs.iter().any(|program| program.custom));
    assert!(
        plan.fallback
            .iter()
            .any(|reason| reason.contains("custom harness"))
    );
}

#[test]
fn declaration() {
    let fixture = Fixture::new();
    write(
        fixture.root.path(),
        "Cargo.toml",
        "[package]\nname='sample'\nversion='0.1.0'\n[lib]\npath='src/../src/lib.rs'\nharness=false\n",
    );
    let plan = fixture.plan().unwrap();
    assert!(!plan.fallback.is_empty());
    assert!(plan.programs.is_empty());
}

#[test]
fn missing() {
    let mut fixture = Fixture::new();
    fixture.add("regular", "test");
    fixture.artifacts.pop();
    let plan = fixture.plan().unwrap();
    assert_eq!(plan.programs.len(), 1);
    assert!(
        plan.fallback
            .iter()
            .any(|reason| reason.contains("regular"))
    );
}

#[test]
fn features() {
    let mut fixture = Fixture::new();
    fixture.add("gated", "test");
    fixture.metadata["packages"][0]["targets"][1]["required-features"] = json!(["extra"]);
    let mut gated = fixture.artifacts.pop().unwrap();
    assert!(fixture.plan().unwrap().fallback.is_empty());
    fixture.artifacts[0]["features"] = json!(["extra"]);
    assert!(!fixture.plan().unwrap().fallback.is_empty());
    gated["features"] = json!(["extra"]);
    fixture.artifacts.push(gated);
    assert_eq!(fixture.plan().unwrap().programs.len(), 2);
    assert!(fixture.plan().unwrap().fallback.is_empty());
}

#[test]
fn docs() {
    let mut fixture = Fixture::new();
    fixture.metadata["packages"][0]["targets"][0]["test"] = json!(false);
    fixture.artifacts.clear();
    let plan = fixture.plan().unwrap();
    assert!(plan.programs.is_empty());
    assert!(plan.docs);
    assert!(plan.fallback.is_empty());
}

#[test]
fn mismatch() {
    let mut fixture = Fixture::new();
    fixture.artifacts[0]["target"]["name"] = json!("foreign");
    assert!(fixture.plan().unwrap_err().contains("does not match"));
}

#[test]
fn duplicate() {
    let mut fixture = Fixture::new();
    fixture.artifacts.push(fixture.artifacts[0].clone());
    assert!(fixture.plan().unwrap_err().contains("duplicate"));
}

#[test]
fn incomplete() {
    let fixture = Fixture::new();
    let metadata = fixture.metadata.to_string();
    let artifact = fixture.artifacts[0].to_string();
    assert!(Plan::read(fixture.root.path(), &metadata, &artifact).is_err());
    let failed = format!("{artifact}\n{{\"reason\":\"build-finished\",\"success\":false}}");
    assert!(Plan::read(fixture.root.path(), &metadata, &failed).is_err());
    let extra = format!("{}\n{artifact}", fixture.stream());
    assert!(Plan::read(fixture.root.path(), &metadata, &extra).is_err());
    assert!(Plan::read(fixture.root.path(), &metadata, "not JSON").is_err());
}

#[test]
fn outside() {
    let mut fixture = Fixture::new();
    fixture.artifacts[0]["executable"] = json!(fixture.root.path().join("src/lib.rs"));
    assert!(
        fixture
            .plan()
            .unwrap_err()
            .contains("leaves its admitted root")
    );
    fixture.metadata["workspace_default_members"] = json!(["foreign"]);
    assert!(
        fixture
            .plan()
            .unwrap_err()
            .contains("default package selection")
    );
}

#[test]
fn defaults() {
    let mut fixture = Fixture::new();
    write(
        fixture.root.path(),
        "member/Cargo.toml",
        "[package]\nname='member'\nversion='0.1.0'\n",
    );
    write(fixture.root.path(), "member/src/lib.rs", "");
    write(fixture.root.path(), "target/debug/deps/member", "");
    let target = json!({"name":"member","kind":["lib"],"src_path":fixture.root.path().join("member/src/lib.rs"),"test":true,"doctest":true});
    fixture.metadata["packages"].as_array_mut().unwrap().push(json!({"id":"member","name":"member","manifest_path":fixture.root.path().join("member/Cargo.toml"),"targets":[target.clone()]}));
    fixture.metadata["workspace_members"] = json!(["sample", "member"]);
    let plan = fixture.plan().unwrap();
    assert_eq!(plan.programs[0].name, "sample");
    fixture.metadata["workspace_default_members"] = json!(["member"]);
    fixture.artifacts.clear();
    fixture.artifacts.push(json!({"reason":"compiler-artifact","package_id":"member","target":target,"profile":{"test":true},"executable":fixture.root.path().join("target/debug/deps/member"),"features":[]}));
    let plan = fixture.plan().unwrap();
    assert_eq!(plan.programs.len(), 1);
    assert_eq!(plan.programs[0].name, "member");
    assert_eq!(
        plan.programs[0].directory,
        fixture.root.path().join("member")
    );
}

#[test]
fn closure() {
    let fixture = Fixture::new();
    let plan = fixture.plan().unwrap();
    let mut metadata = fixture.metadata.clone();
    assert_eq!(plan.groups(&metadata.to_string()).unwrap().len(), 1);
    let mut foreign = metadata["packages"][0].clone();
    foreign["id"] = json!("other");
    foreign["name"] = json!("other");
    foreign["targets"][0]["test"] = json!(false);
    metadata["packages"].as_array_mut().unwrap().push(foreign);
    metadata["workspace_default_members"] = json!(["sample", "other"]);
    assert!(
        plan.groups(&metadata.to_string())
            .err()
            .unwrap()
            .contains("unqualified")
    );
    metadata["workspace_default_members"] = json!(["sample"]);
    assert!(plan.groups(&metadata.to_string()).is_ok());
}

#[test]
fn wildcard() {
    let mut fixture = Fixture::new();
    fixture.add("regular", "test");
    let mut plan = fixture.plan().unwrap();
    plan.programs
        .iter_mut()
        .find(|program| program.target.name == "regular")
        .unwrap()
        .target
        .name = "regular*".into();
    assert!(
        plan.groups(&fixture.metadata.to_string())
            .err()
            .unwrap()
            .contains("exact Cargo selector")
    );
}
