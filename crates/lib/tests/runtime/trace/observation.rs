use plumb::observation::{self, Declaration, Run};
use serde_json::Value;
use std::path::Path;

struct Home(tempfile::TempDir);

impl Home {
    fn new() -> Self {
        let home = Self(tempfile::tempdir().expect("home"));
        std::fs::write(
            home.file(),
            "[locus]\nenabled = true\nendpoint = 'http://127.0.0.1:0'\n",
        )
        .expect("configuration");
        home
    }

    fn file(&self) -> std::path::PathBuf {
        self.0.path().join("plumb.toml")
    }

    fn start(&self, command: &str) -> Run {
        let file = self.file();
        let declaration = Declaration {
            producer: "plumb",
            file: &file,
            buffer: self.0.path().join("state/locus"),
        };
        Run::start(declaration, command).expect("run")
    }

    fn records(&self) -> Vec<Value> {
        records(&self.0.path().join("state/locus"))
    }
}

fn records(buffer: &Path) -> Vec<Value> {
    let mut records = Vec::new();
    for entry in std::fs::read_dir(buffer).expect("buffer") {
        let path = entry.expect("entry").path();
        let name = path.file_name().expect("name").to_string_lossy();
        if name == "active.jsonl" || name.starts_with("sealed-") {
            let content = std::fs::read_to_string(path).expect("records");
            records.extend(
                content
                    .lines()
                    .map(|line| serde_json::from_str::<Value>(line).expect("atom")),
            );
        }
    }
    records
}

#[locus::trace(with = observation::view())]
fn active() {}

#[locus::trace(with = run.map(Run::view))]
fn scoped(run: Option<&Run>) {}

fn edges(atoms: &[Value]) -> usize {
    atoms
        .iter()
        .filter(|atom| atom["source"]["edge"] == "enter" || atom["source"]["edge"] == "return")
        .count()
}

fn finishes(atoms: &[Value]) -> Vec<(Value, Value)> {
    atoms
        .iter()
        .filter(|atom| atom["payload"]["event"] == "cli.finish")
        .map(|atom| {
            (
                atom["payload"]["code"].clone(),
                atom["context"]["plumb.fault"].clone(),
            )
        })
        .collect()
}
#[test]
fn repeated() {
    sequential();
    overlapping();
}

fn sequential() {
    let first = Home::new();
    let second = Home::new();

    let run = first.start("doctor");
    active();
    observation::fault("doctor.first");
    run.finish(1);
    assert!(
        observation::view().is_none(),
        "finished run is no longer active"
    );
    active();
    observation::fault("stray");

    let run = second.start("plan");
    active();
    scoped(Some(&run));
    run.fault("plan.second");
    run.finish(1);

    let doctor = first.records();
    let plan = second.records();
    assert!(
        doctor
            .iter()
            .all(|atom| atom["context"]["plumb.command"] == "doctor")
    );
    assert!(
        plan.iter()
            .all(|atom| atom["context"]["plumb.command"] == "plan")
    );
    assert_eq!(doctor.len(), 5);
    assert_eq!(plan.len(), 7);
    assert_eq!(edges(&doctor), 2);
    assert_eq!(edges(&plan), 4);
    assert_eq!(
        finishes(&doctor),
        vec![(Value::from(1), Value::from("doctor.first"))]
    );
    assert_eq!(
        finishes(&plan),
        vec![(Value::from(1), Value::from("plan.second"))]
    );
}

fn overlapping() {
    let outer = Home::new();
    let inner = Home::new();
    let first = outer.start("outer");
    let second = inner.start("inner");
    scoped(Some(&first));
    first.fault("outer.fault");
    observation::fault("inner.fault");
    second.finish(2);
    first.finish(3);
    assert!(observation::view().is_none());
    let outer = outer.records();
    let inner = inner.records();
    assert!(
        outer
            .iter()
            .all(|atom| atom["context"]["plumb.command"] == "outer")
    );
    assert!(
        inner
            .iter()
            .all(|atom| atom["context"]["plumb.command"] == "inner")
    );
    assert_eq!(edges(&outer), 2);
    assert_eq!(
        finishes(&outer),
        vec![(Value::from(3), Value::from("outer.fault"))]
    );
    assert_eq!(
        finishes(&inner),
        vec![(Value::from(2), Value::from("inner.fault"))]
    );
}
