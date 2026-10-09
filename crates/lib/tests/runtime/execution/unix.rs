use plumb::config::{Contract, Execution};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

fn tool(root: &Path, body: &str) {
    let path = root.join("probe");
    std::fs::write(&path, body).expect("tool");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("executable");
}

fn execution(root: &Path, paths: &[&Path]) -> Result<Execution, String> {
    let path = std::env::join_paths(paths).expect("PATH");
    let environment = Contract {
        inherit: vec!["PATH".into()],
        managed: vec![],
        reject: vec![],
        bind: Default::default(),
    }
    .capture([("PATH".into(), path)])?;
    Execution::new(environment, &["probe".into()], root)
}

#[test]
fn unrelated() {
    let root = tempfile::tempdir().expect("tools");
    let unrelated = tempfile::tempdir().expect("unrelated tools");
    tool(root.path(), "#!/bin/sh\nprintf '%s' \"$PATH\"\n");
    let first = execution(root.path(), &[root.path()]).expect("execution");
    let second = execution(root.path(), &[unrelated.path(), root.path()]).expect("execution");
    assert_eq!(first.evidence().unwrap(), second.evidence().unwrap());
    let output = second.command("probe").unwrap().output().unwrap();
    assert!(output.status.success());
    assert_eq!(
        output.stdout,
        second.environment.get("PATH").unwrap().as_bytes()
    );
}

#[test]
fn replaced() {
    let root = tempfile::tempdir().expect("tools");
    tool(root.path(), "#!/bin/sh\necho same-version\n");
    let first = execution(root.path(), &[root.path()]).expect("execution");
    tool(root.path(), "#!/bin/sh\nprintf '%s\\n' same-version\n");
    let second = execution(root.path(), &[root.path()]).expect("execution");
    assert_ne!(first.evidence().unwrap(), second.evidence().unwrap());
    assert!(
        first
            .command("probe")
            .unwrap_err()
            .contains("changed before execution")
    );
}

#[test]
fn shadowed() {
    let root = tempfile::tempdir().expect("tools");
    let shadow = tempfile::tempdir().expect("shadow tools");
    tool(root.path(), "#!/bin/sh\necho same-version\n");
    let first = execution(root.path(), &[shadow.path(), root.path()]).expect("execution");
    tool(shadow.path(), "#!/bin/sh\necho same-version\n");
    let second = execution(root.path(), &[shadow.path(), root.path()]).expect("execution");
    assert_ne!(first.evidence().unwrap(), second.evidence().unwrap());
    assert!(
        first
            .command("probe")
            .unwrap_err()
            .contains("changed before execution")
    );
}

#[test]
fn proxy() {
    let root = tempfile::tempdir().expect("tools");
    std::fs::create_dir(root.path().join("bin")).unwrap();
    tool(&root.path().join("bin"), "#!/bin/sh\nprintf '%s' \"$0\"\n");
    std::fs::rename(
        root.path().join("bin/probe"),
        root.path().join("bin/actual"),
    )
    .unwrap();
    std::os::unix::fs::symlink("actual", root.path().join("bin/probe")).unwrap();
    let held = execution(root.path(), &[Path::new("bin")]).expect("relative execution");
    let mut command = held.command("probe").unwrap();
    assert_eq!(command.get_program(), root.path().join("bin/probe"));
    let output = command.output().unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        root.path().join("bin/probe").to_str().unwrap()
    );
    assert!(
        held.command("absent")
            .unwrap_err()
            .contains("no resolved tool")
    );
}

#[test]
fn absent() {
    let root = tempfile::tempdir().expect("tools");
    assert!(
        execution(root.path(), &[root.path()])
            .err()
            .unwrap()
            .contains("cannot resolve tool probe")
    );
}

#[test]
fn bindings() {
    let root = tempfile::tempdir().unwrap();
    tool(root.path(), "#!/bin/sh\nprintf '%s' \"$WRAPPER\"\n");
    let contract: Contract = toml::from_str("inherit=['PATH']\nmanaged=[]\nreject=['WRAPPER']\n[bind]\nWRAPPER={tool='probe'}\nINCREMENTAL='0'\n").unwrap();
    let capture = |wrapper: &str| {
        contract
            .capture([
                ("PATH".into(), root.path().as_os_str().to_owned()),
                ("WRAPPER".into(), wrapper.into()),
            ])
            .unwrap()
    };
    let first = Execution::new(capture("probe"), &[], root.path()).unwrap();
    let path = root.path().join("probe");
    let second = Execution::new(capture(path.to_str().unwrap()), &[], root.path()).unwrap();
    assert_eq!(first.evidence().unwrap(), second.evidence().unwrap());
    assert_eq!(first.imprint().unwrap(), second.imprint().unwrap());
    let command = first.command("probe").unwrap();
    assert!(
        command
            .get_envs()
            .any(|(key, value)| key == "INCREMENTAL" && value == Some(std::ffi::OsStr::new("0")))
    );
    let output = first.output(&["probe".into()]).unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, path.to_str().unwrap().as_bytes());
    assert!(
        Execution::new(capture("/private/probe"), &[], root.path())
            .err()
            .unwrap()
            .contains("differs from its resolved tool")
    );
    tool(root.path(), "#!/bin/sh\necho changed\n");
    assert!(
        first
            .command("probe")
            .err()
            .unwrap()
            .contains("changed before execution")
    );
}
