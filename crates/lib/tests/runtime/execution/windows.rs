use super::fixture;
use plumb::config::{Contract, Execution, Search};
use std::path::Path;

fn batch(root: &Path) {
    let binary = fixture::binary().join("probe.exe");
    std::fs::write(
        root.join("probe.cmd"),
        format!("@echo off\r\n\"{}\" %*\r\n", binary.display()),
    )
    .unwrap();
}

#[test]
fn arguments() {
    let root = tempfile::Builder::new()
        .prefix("batch 中文 ")
        .tempdir()
        .unwrap();
    batch(root.path());
    let arguments = fixture::arguments();
    let output = fixture::search(root.path())
        .command("probe")
        .unwrap()
        .env("TOKEN", "EXPANDED")
        .args(arguments)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        format!("{arguments:?}")
    );
    let error = fixture::search(root.path())
        .command("probe.cmd")
        .unwrap()
        .arg("line\nvalue")
        .spawn()
        .unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(
        fixture::search(root.path())
            .command("probe")
            .unwrap()
            .arg("--exit")
            .status()
            .unwrap()
            .code(),
        Some(37)
    );
}

#[test]
fn extensions() {
    let root = tempfile::tempdir().unwrap();
    batch(root.path());
    fixture::tool(root.path(), "probe.exe");
    let path = Some(root.path().as_os_str().to_owned());
    let first = Search::new(root.path(), path.clone(), Some(".CMD;.EXE".into())).unwrap();
    let second = Search::new(root.path(), path.clone(), Some(".EXE;.CMD".into())).unwrap();
    assert!(
        first
            .resolve("probe")
            .unwrap()
            .extension()
            .unwrap()
            .eq_ignore_ascii_case("CMD")
    );
    assert!(
        second
            .resolve("probe")
            .unwrap()
            .extension()
            .unwrap()
            .eq_ignore_ascii_case("EXE")
    );
    assert!(
        first
            .resolve("probe")
            .unwrap()
            .extension()
            .unwrap()
            .eq_ignore_ascii_case("CMD")
    );
    assert!(
        Search::new(root.path(), path, None)
            .unwrap()
            .resolve("probe")
            .is_err()
    );
}

#[test]
fn ordering() {
    let root = tempfile::tempdir().unwrap();
    let next = tempfile::tempdir().unwrap();
    batch(root.path());
    fixture::tool(next.path(), "probe.exe");
    let path = std::env::join_paths([root.path(), next.path()]).unwrap();
    let search = Search::new(root.path(), Some(path), Some(".EXE;.CMD".into())).unwrap();
    assert_eq!(
        search
            .resolve("probe")
            .unwrap()
            .parent()
            .unwrap()
            .canonicalize()
            .unwrap(),
        root.path().canonicalize().unwrap()
    );
}

#[test]
fn unsupported() {
    let root = tempfile::tempdir().unwrap();
    fixture::tool(root.path(), "probe.ps1");
    let search = Search::new(
        root.path(),
        Some(root.path().as_os_str().to_owned()),
        Some(".PS1".into()),
    )
    .unwrap();
    assert!(
        search
            .command("probe")
            .unwrap_err()
            .contains("unsupported executable format")
    );
}

#[test]
fn explicit() {
    let root = tempfile::tempdir().unwrap();
    batch(root.path());
    let target = root.path().join("probe.bat");
    std::fs::rename(root.path().join("probe.cmd"), &target).unwrap();
    let search = Search::new(root.path(), None, None).unwrap();
    assert_eq!(
        search
            .command(target)
            .unwrap()
            .arg("--exit")
            .status()
            .unwrap()
            .code(),
        Some(37)
    );
}

fn execution(root: &Path) -> Execution {
    let contract = Contract {
        inherit: vec!["PATH".into(), "PATHEXT".into()],
        managed: vec![],
        reject: vec![],
        bind: Default::default(),
    };
    let environment = contract
        .capture([
            ("PATH".into(), root.as_os_str().to_owned()),
            ("PATHEXT".into(), ".CMD;.EXE".into()),
        ])
        .unwrap();
    Execution::new(environment, &["probe".into()], root).unwrap()
}

#[test]
fn replaced() {
    let root = tempfile::tempdir().unwrap();
    batch(root.path());
    let held = execution(root.path());
    assert_eq!(
        held.command("probe").unwrap().get_program(),
        held.tools().unwrap()["probe"].path.as_os_str()
    );
    std::fs::write(root.path().join("probe.cmd"), "@exit /b 37").unwrap();
    assert!(
        held.command("probe")
            .unwrap_err()
            .contains("changed before execution")
    );
}

#[test]
fn shadowed() {
    let root = tempfile::tempdir().unwrap();
    fixture::tool(root.path(), "probe.exe");
    let held = execution(root.path());
    batch(root.path());
    assert!(
        held.command("probe")
            .unwrap_err()
            .contains("changed before execution")
    );
}
