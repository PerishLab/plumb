use locus::{Atom, Origin};
use std::fs;
use std::process::Command;

#[test]
fn atoms() {
    let home = tempfile::tempdir().expect("temp");
    let trace = home.path().join("trace.json");
    let report = home.path().join("audit.jsonl");
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["doctor", home.path().to_str().expect("path")])
        .env("PLUMB_LOCUS_TRACE_FILE", &trace)
        .env("PLUMB_LOCUS_REPORT_FILE", &report)
        .output()
        .expect("plumb");

    assert!(output.status.success());
    let text = fs::read_to_string(report).expect("report");
    let atoms: Vec<Atom> = text
        .lines()
        .map(|line| serde_json::from_str(line).expect("atom"))
        .collect();
    assert_eq!(atoms.len(), 2);
    assert_eq!(atoms[0].payload().expect("payload")["event"], "cli.start");
    assert_eq!(atoms[1].payload().expect("payload")["event"], "cli.finish");
    assert_eq!(atoms[0].payload().expect("payload")["command"], "doctor");
    assert_eq!(atoms[1].payload().expect("payload")["code"], 0);
    assert_eq!(atoms[0].context(), atoms[1].context());
    assert_eq!(atoms[0].choices().len(), 2);
    assert_eq!(atoms[1].choices().len(), 2);
    assert!(
        atoms[0]
            .choices()
            .iter()
            .all(|choice| choice.origin() == &Origin::Generated)
    );
    assert!(
        atoms[1]
            .choices()
            .iter()
            .all(|choice| choice.origin() == &Origin::Inherited)
    );
    assert!(
        atoms[0]
            .source()
            .expect("source")
            .file()
            .ends_with("main.rs")
    );
    assert!(trace.is_file());
}

#[test]
fn explicit() {
    let home = tempfile::tempdir().expect("temp");
    let report = home.path().join("audit.jsonl");
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .arg("--version")
        .env("PLUMB_LOCUS_TRACE_ID", "manual-trace")
        .env("PLUMB_LOCUS_REPORT_FILE", &report)
        .output()
        .expect("plumb");

    assert!(output.status.success());
    let text = fs::read_to_string(report).expect("report");
    let atom: Atom = serde_json::from_str(text.lines().next().expect("start")).expect("atom");
    let trace = atom
        .choices()
        .iter()
        .find(|choice| choice.role() == "locus.trace")
        .expect("trace");
    assert_eq!(trace.key(), "manual-trace");
    assert_eq!(trace.origin(), &Origin::Explicit);
}
