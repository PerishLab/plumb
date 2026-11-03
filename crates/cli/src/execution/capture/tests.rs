use super::configuration::{configured, foreign};
use super::*;
use std::process::Command;

#[test]
fn record() {
    let arguments = std::env::args_os()
        .skip_while(|argument| argument != "--")
        .skip(1)
        .collect::<Vec<_>>();
    if let Some((root, arguments)) = arguments.split_first() {
        assert_eq!(run(Path::new(root), arguments), 0);
        return;
    }
    let executable = std::env::current_exe().unwrap();
    let root = std::env::current_dir().unwrap();
    let carrier = vec![executable.to_str().unwrap().to_string()];
    let session = Session::new([(executable.clone(), root)].into_iter(), &carrier).unwrap();
    let configuration = session.configuration(&carrier).unwrap();
    assert!(configuration.contains("cfg(all())"));
    assert!(session.contexts().is_err());
    let parsed: toml::Value = toml::from_str(&configuration).unwrap();
    let root = Path::new(
        parsed["target"]["cfg(all())"]["runner"][1]
            .as_str()
            .unwrap(),
    );
    assert_eq!(run(root, &[executable.clone().into_os_string()]), 0);
    assert_ne!(run(root, &[executable.clone().into_os_string()]), 0);
    let contexts = session.contexts().unwrap();
    let command = contexts[0].command(1).unwrap();
    let actual = command
        .get_envs()
        .filter_map(|(key, value)| value.map(|value| (key.to_os_string(), value.to_os_string())))
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(actual, std::env::vars_os().collect());
    let invalid = tempfile::tempdir().unwrap();
    assert_ne!(run(invalid.path(), &[executable.into_os_string()]), 0);
}

#[test]
fn configuration() {
    for text in [
        "[target.'cfg(all())']\nrunner=['custom']",
        "[build]\ntarget='foreign'",
        "include='other.toml'",
        "target-applies-to-host=false",
        "[env]\nCARGO_TARGET_HOST_RUNNER='custom'",
    ] {
        assert!(configured(&toml::from_str(text).unwrap()));
    }
    assert!(!configured(
        &toml::from_str("[registries.perish]\nindex='sparse+https://cargo.perish.uk/'").unwrap()
    ));
}

#[test]
fn inherited() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().join("source");
    let home = fixture.path().join("home");
    std::fs::create_dir(&root).unwrap();
    std::fs::create_dir(&home).unwrap();
    let mut command = Command::new("cargo");
    command.env("CARGO_HOME", &home);
    assert!(!foreign(&root, &command).unwrap());
    std::fs::write(
        home.join("config.toml"),
        "[target.'cfg(all())']\nrunner='custom'",
    )
    .unwrap();
    assert!(foreign(&root, &command).unwrap());
    std::fs::remove_file(home.join("config.toml")).unwrap();
    std::fs::create_dir(fixture.path().join(".cargo")).unwrap();
    std::fs::write(
        fixture.path().join(".cargo/config"),
        "[build]\ntarget='foreign'",
    )
    .unwrap();
    assert!(foreign(&root, &command).unwrap());
}

#[test]
fn descriptors() {
    let executable = std::env::current_exe().unwrap();
    let tool = plumb::config::Tool::at(executable).unwrap();
    for (key, value) in [
        ("CARGO_MAKEFLAGS", "--jobserver-auth=3,4"),
        ("LISTEN_FDS", "2"),
        ("invalid-key", "value"),
    ] {
        let context: Context = serde_json::from_value(serde_json::json!({
            "tool":tool,"directory":std::env::current_dir().unwrap(),
            "arguments":[],"environment":{key:value}
        }))
        .unwrap();
        assert!(context.command(1).is_err());
    }
}

#[test]
fn admission() {
    use std::os::unix::fs::PermissionsExt;
    let executable = std::env::current_exe().unwrap();
    let carrier = vec![executable.to_str().unwrap().to_string()];
    let session = Session::new(
        [(executable.clone(), std::env::current_dir().unwrap())].into_iter(),
        &carrier,
    )
    .unwrap();
    let parsed: toml::Value = toml::from_str(&session.configuration(&carrier).unwrap()).unwrap();
    let root = Path::new(
        parsed["target"]["cfg(all())"]["runner"][1]
            .as_str()
            .unwrap(),
    );
    assert_eq!(
        std::fs::metadata(root).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(run(root, &[executable.clone().into_os_string()]), 0);
    let path = root.join("0.json");
    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let captured = std::fs::read(&path).unwrap();
    std::fs::write(root.join("extra.json"), "{}").unwrap();
    assert!(session.contexts().is_err());
    std::fs::remove_file(root.join("extra.json")).unwrap();
    let mut changed: serde_json::Value = serde_json::from_slice(&captured).unwrap();
    changed["directory"] = serde_json::json!(root);
    std::fs::write(&path, serde_json::to_vec(&changed).unwrap()).unwrap();
    assert!(session.contexts().is_err());
    std::fs::remove_file(&path).unwrap();
    std::os::unix::fs::symlink(root.join("session.json"), &path).unwrap();
    assert!(session.contexts().is_err());
    let declaration = root.join("session.json");
    let mut changed: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&declaration).unwrap()).unwrap();
    changed["runner"]["path"] = serde_json::json!(root.join("foreign"));
    std::fs::write(&declaration, serde_json::to_vec(&changed).unwrap()).unwrap();
    assert_ne!(run(root, &[executable.into_os_string()]), 0);
}
