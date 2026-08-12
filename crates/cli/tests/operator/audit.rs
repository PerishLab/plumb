use locus::{Atom, Origin};
use std::fs;
use std::process::Command;

#[test]
fn atoms() {
    let home = tempfile::tempdir().expect("temp");
    let trace = home.path().join("trace.json");
    let report = home.path().join("audit.jsonl");
    let output = plumb()
        .args(["doctor", home.path().to_str().expect("path")])
        .env("PLUMB_LOCUS_ENABLED", "true")
        .env("PLUMB_LOCUS_TRACE_FILE", &trace)
        .env("PLUMB_LOCUS_REPORT_FILE", &report)
        .output()
        .expect("plumb");

    assert!(output.status.success());
    let atoms = read(&report);
    assert_eq!(atoms.len(), 2);
    assert_eq!(atoms[0].payload().expect("payload")["event"], "cli.start");
    assert_eq!(atoms[1].payload().expect("payload")["event"], "cli.finish");
    assert_eq!(atoms[0].payload().expect("payload")["command"], "doctor");
    assert_eq!(atoms[1].payload().expect("payload")["code"], 0);
    assert_eq!(atoms[0].context(), atoms[1].context());
    assert!(!atoms[0].context().contains_key("plumb.target"));
    assert!(atoms[0].collections().is_empty());
    assert!(atoms[1].collections().is_empty());
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
            .ends_with("audit.rs")
    );
    assert!(trace.is_file());
}

#[test]
fn explicit() {
    let home = tempfile::tempdir().expect("temp");
    let report = home.path().join("audit.jsonl");
    let output = plumb()
        .arg("--version")
        .env("PLUMB_LOCUS_ENABLED", "true")
        .env("PLUMB_LOCUS_TRACE_ID", "manual-trace")
        .env("PLUMB_LOCUS_REPORT_FILE", &report)
        .env("CODEX_THREAD_ID", "ignored-thread")
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
    assert!(atom.collections().is_empty());
}

#[test]
fn identity() {
    let home = tempfile::tempdir().expect("temp");
    let trace = home.path().join("trace.json");
    let report = home.path().join("audit.jsonl");
    let output = plumb()
        .args(["doctor", home.path().to_str().expect("path")])
        .env("PLUMB_LOCUS_ENABLED", "true")
        .env("PLUMB_LOCUS_TRACE_FILE", &trace)
        .env("PLUMB_LOCUS_REPORT_FILE", &report)
        .env("CODEX_THREAD_ID", "codex-thread")
        .output()
        .expect("plumb");

    assert!(output.status.success());
    let atoms = read(&report);
    assert_eq!(atoms.len(), 2);
    assert!(
        atoms
            .iter()
            .all(|atom| atom.context()["locus.trace"] == "codex-thread")
    );
    let collected = &atoms[0].collections()[0];
    assert_eq!(collected.role(), "locus.trace");
    assert_eq!(collected.binding(), "codex.thread");
    assert_eq!(collected.collector(), "environment");
    assert_eq!(collected.selector(), "CODEX_THREAD_ID");
    assert!(atoms[1].collections().is_empty());
    assert!(!trace.exists());
}

#[test]
fn muted() {
    for enabled in [None, Some("false")] {
        let home = tempfile::tempdir().expect("temp");
        let trace = home.path().join("trace.json");
        let report = home.path().join("audit.jsonl");
        let mut command = plumb();
        command
            .args(["doctor", home.path().to_str().expect("path")])
            .env_remove("PLUMB_LOCUS_ENABLED")
            .env("PLUMB_LOCUS_TRACE_FILE", &trace)
            .env("PLUMB_LOCUS_REPORT_FILE", &report)
            .env("PLUMB_LOCUS_TARGET_COLLECTORS", "process:parent");
        if let Some(value) = enabled {
            command.env("PLUMB_LOCUS_ENABLED", value);
        }
        let output = command.output().expect("plumb");
        assert!(output.status.success());
        assert!(!trace.exists());
        assert!(!report.exists());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("locus engine diagnostic"));
    }
}

#[test]
fn malformed() {
    let home = tempfile::tempdir().expect("temp");
    let report = home.path().join("audit.jsonl");
    let output = plumb()
        .args(["doctor", home.path().to_str().expect("path")])
        .env("PLUMB_LOCUS_ENABLED", "yes")
        .env("PLUMB_LOCUS_REPORT_FILE", &report)
        .output()
        .expect("plumb");

    assert!(output.status.success());
    assert!(!report.exists());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("cannot parse PLUMB_LOCUS_ENABLED: neither true nor false")
    );
}

#[test]
fn collectors() {
    let home = tempfile::tempdir().expect("temp");
    let atoms = invoke(
        home.path(),
        "environment:PLUMB_AUDIT_TARGET",
        Some(("PLUMB_AUDIT_TARGET", "repository-a")),
    );
    assert_eq!(atoms[0].context()["plumb.target"], "repository-a");
    assert_eq!(atoms[1].context()["plumb.target"], "repository-a");
    assert_eq!(atoms[0].collections()[0].role(), "plumb.target");
    assert_eq!(atoms[0].collections()[0].binding(), "target");
    assert_eq!(atoms[0].collections()[0].collector(), "environment");
    assert_eq!(atoms[0].collections()[0].selector(), "PLUMB_AUDIT_TARGET");
    assert!(atoms[1].collections().is_empty());

    let home = tempfile::tempdir().expect("temp");
    let atoms = invoke(home.path(), "argv:1", None);
    assert_eq!(
        atoms[0].context()["plumb.target"],
        home.path().to_string_lossy()
    );
    assert_eq!(atoms[0].collections()[0].collector(), "argv");
    assert_eq!(atoms[0].collections()[0].selector(), "1");

    let home = tempfile::tempdir().expect("temp");
    let atoms = invoke(
        home.path(),
        "environment:PLUMB_AUDIT_MISSING,process:id",
        None,
    );
    assert!(atoms[0].context()["plumb.target"].parse::<u32>().is_ok());
    assert_eq!(atoms[0].collections()[0].collector(), "process");
    assert_eq!(atoms[0].collections()[0].selector(), "id");
}

#[test]
fn invalid() {
    let home = tempfile::tempdir().expect("temp");
    let report = home.path().join("audit.jsonl");
    let output = plumb()
        .args(["doctor", home.path().to_str().expect("path")])
        .env("PLUMB_LOCUS_ENABLED", "true")
        .env("PLUMB_LOCUS_TARGET_COLLECTORS", "process:parent")
        .env("PLUMB_LOCUS_REPORT_FILE", &report)
        .output()
        .expect("plumb");

    assert!(output.status.success());
    assert!(!report.exists());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("unknown process collector selector: parent")
    );
}

fn invoke(root: &std::path::Path, collectors: &str, value: Option<(&str, &str)>) -> Vec<Atom> {
    let report = root.join("audit.jsonl");
    let mut command = plumb();
    command
        .args(["doctor", root.to_str().expect("path")])
        .env("PLUMB_LOCUS_ENABLED", "true")
        .env("PLUMB_LOCUS_TRACE_ID", "collector-trace")
        .env("PLUMB_LOCUS_TARGET_COLLECTORS", collectors)
        .env("PLUMB_LOCUS_REPORT_FILE", &report)
        .env_remove("PLUMB_AUDIT_MISSING");
    if let Some((name, held)) = value {
        command.env(name, held);
    }
    let output = command.output().expect("plumb");
    assert!(output.status.success());
    read(&report)
}

fn plumb() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_plumb"));
    command.env_remove("CODEX_THREAD_ID");
    command
}

fn read(path: &std::path::Path) -> Vec<Atom> {
    fs::read_to_string(path)
        .expect("report")
        .lines()
        .map(|line| serde_json::from_str(line).expect("atom"))
        .collect()
}
