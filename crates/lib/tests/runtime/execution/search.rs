use super::fixture;
use plumb::config::Search;

#[test]
fn arguments() {
    let root = tempfile::tempdir().unwrap();
    fixture::tool(root.path(), "probe.exe");
    let arguments = fixture::arguments();
    let output = fixture::search(root.path())
        .command("probe.exe")
        .unwrap()
        .args(arguments)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        format!("{arguments:?}")
    );
}

#[test]
fn relative() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("bin")).unwrap();
    fixture::tool(&root.path().join("bin"), "probe.exe");
    let search = Search::new(root.path(), Some("bin".into()), Some(".EXE".into())).unwrap();
    let expected = root.path().canonicalize().unwrap().join("bin/probe.exe");
    assert_eq!(search.resolve("probe.exe").unwrap(), expected);
    assert_eq!(search.resolve("bin/probe.exe").unwrap(), expected);
    let mut command = search.command("probe.exe").unwrap();
    assert_eq!(
        command.get_current_dir().unwrap(),
        root.path().canonicalize().unwrap()
    );
    assert!(command.status().unwrap().success());
}

#[test]
fn absent() {
    let root = tempfile::tempdir().unwrap();
    assert!(
        fixture::search(root.path())
            .resolve("missing")
            .unwrap_err()
            .contains("cannot resolve tool")
    );
    assert!(Search::new(&root.path().join("missing"), None, None).is_err());
    let file = root.path().join("file");
    std::fs::write(&file, "content").unwrap();
    assert!(Search::new(&file, None, None).is_err());
}

#[test]
fn empty() {
    let root = tempfile::tempdir().unwrap();
    fixture::tool(root.path(), "probe.exe");
    let search = Search::new(root.path(), Some("".into()), Some(".EXE".into())).unwrap();
    if cfg!(windows) {
        assert!(search.resolve("probe.exe").is_err());
    } else {
        assert!(
            search
                .command("probe.exe")
                .unwrap()
                .status()
                .unwrap()
                .success()
        );
    }
}

#[test]
fn explicit() {
    let root = tempfile::tempdir().unwrap();
    let tool = fixture::tool(root.path(), "probe.exe");
    let search = Search::new(root.path(), None, None).unwrap();
    assert!(search.command(tool).unwrap().status().unwrap().success());
    assert!(search.resolve("probe.exe").is_err());
}

#[test]
fn status() {
    let root = tempfile::tempdir().unwrap();
    fixture::tool(root.path(), "probe.exe");
    let status = fixture::search(root.path())
        .command("probe.exe")
        .unwrap()
        .arg("--exit")
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(37));
}

#[test]
fn environment() {
    let root = tempfile::tempdir().unwrap();
    fixture::tool(root.path(), "probe.exe");
    let command = fixture::search(root.path()).command("probe.exe").unwrap();
    assert!(
        command
            .get_envs()
            .any(|(key, value)| key == "PATH" && value == Some(root.path().as_os_str()))
    );
    assert!(!command.get_envs().any(|(key, _)| key == "NODE_OPTIONS"));
}
