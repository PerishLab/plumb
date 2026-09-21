use super::{Repo, cache, support};

#[test]
fn bootstrap() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let held = std::fs::read_to_string(
        root.join("crates/cli/src/command/guard/precommit/configuration.rs"),
    )
    .expect("configuration bootstrap");
    assert!(held.contains("Context::Source => source.latest(\"stable\", true)?"));
    assert!(held.contains("Ok(branch) if branch.starts_with(\"release/\")"));
    let action =
        std::fs::read_to_string(root.join("crates/cli/src/command/guard/precommit/action.rs"))
            .expect("precommit action");
    let mismatch = action.find("let mismatched").expect("mismatch decision");
    let reuse = action
        .find("if !mismatched")
        .expect("conditioned proof reuse");
    assert!(mismatch < reuse);
    assert!(action.contains("configuration::mismatched(target.as_deref())?"));
    assert!(held.contains("plumb::depot::related(target, released)"));
    assert!(action.find(".checks()?").expect("current actions") < reuse);
    let selection = action
        .find("configuration.product(root)?")
        .expect("verified profile");
    assert!(action.find("Seat::new(").expect("configuration validation") < selection);
    assert!(selection < action.find(".checks()?").unwrap());
    assert!(!held.contains("Spec::read"));
    assert!(!held.contains("product != Some(\"plumb\")"));
    assert!(held.contains("crate::shape::depot::governed(&snapshot, profile)?"));
    assert!(
        action.find("crate::shape::product::guard").unwrap()
            < action.find("configuration::target").unwrap()
    );
}

#[test]
fn unchanged() {
    let fixture = cache::fixture();
    let home = support::depot(&[]);
    let first = cache::run(fixture.path(), home.path());
    cache::success(&first);
    let second = cache::run(fixture.path(), home.path());
    cache::success(&second);
    assert_eq!(first.stdout, second.stdout);
    assert!(!String::from_utf8_lossy(&second.stderr).contains("guard guard/rust"));
}

#[test]
fn unrelated() {
    let fixture = cache::fixture();
    let home = support::depot(&[]);
    let first = cache::run(fixture.path(), home.path());
    cache::success(&first);
    let extra = tempfile::tempdir().expect("unrelated tools");
    let mut paths = vec![extra.path().to_path_buf()];
    paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").expect("PATH"),
    ));
    let second = std::process::Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["guard", ".", "--json"])
        .current_dir(fixture.path())
        .env("PLUMB_HOME", home.path())
        .env("PATH", std::env::join_paths(paths).expect("PATH"))
        .output()
        .expect("guard");
    cache::success(&second);
    assert_eq!(first.stdout, second.stdout);
    assert!(!String::from_utf8_lossy(&second.stderr).contains("guard guard/rust"));
}

#[test]
#[cfg(unix)]
fn bounded() {
    use std::os::unix::fs::PermissionsExt as _;
    let fixture = cache::fixture();
    let home = support::depot(&[]);
    cache::success(&cache::run(fixture.path(), home.path()));
    let tools = tempfile::tempdir().expect("tools");
    let rustc = tools.path().join("rustc");
    let mut paths = vec![tools.path().to_path_buf()];
    paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").expect("PATH"),
    ));
    let path = std::env::join_paths(paths).expect("PATH");
    for (body, expected) in [
        ("while :; do printf x; done", "65536"),
        ("while :; do :; done", "5 second"),
        ("printf '\\377'", "not UTF-8"),
    ] {
        std::fs::write(&rustc, format!("#!/bin/sh\n{body}\n")).expect("probe tool");
        std::fs::set_permissions(&rustc, std::fs::Permissions::from_mode(0o755)).unwrap();
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_plumb"))
            .args(["guard", ".", "--json"])
            .current_dir(fixture.path())
            .env("PLUMB_HOME", home.path())
            .env("PATH", &path)
            .output()
            .expect("guard");
        assert!(!output.status.success());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains(expected), "{error}");
        assert!(!error.contains("guard guard/rust"), "{error}");
    }
}

#[test]
#[cfg(unix)]
fn unread() {
    use std::os::unix::fs::PermissionsExt as _;
    let version = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .expect("real rustc");
    assert!(version.status.success());
    let version = String::from_utf8(version.stdout).expect("version");
    let fixture = cache::fixture();
    let root = fixture.path();
    let home = support::depot(&[]);
    cache::success(&cache::run(root, home.path()));
    let tools = tempfile::tempdir().expect("tools");
    let rustc = tools.path().join("rustc");
    std::fs::write(&rustc, "#!/bin/sh\nexit 71\n").expect("unread tool");
    std::fs::set_permissions(&rustc, std::fs::Permissions::from_mode(0o755)).expect("executable");
    let mut paths = vec![tools.path().to_path_buf()];
    paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").expect("PATH"),
    ));
    let path = std::env::join_paths(paths).expect("tool path");
    for changed in [false, true] {
        if changed {
            std::fs::write(root.join("NOTES"), "unrelated\n").expect("unrelated");
            Repo::git(root, &["add", "NOTES"]);
        }
        let mut guard = std::process::Command::new(env!("CARGO_BIN_EXE_plumb"));
        guard
            .args(["guard", ".", "--json"])
            .current_dir(root)
            .env("PLUMB_HOME", home.path())
            .env("PATH", &path);
        std::fs::write(&rustc, "#!/bin/sh\nexit 71\n").expect("unread tool");
        let output = guard.output().expect("guard");
        assert!(!output.status.success(), "unread tool cannot reuse proof");
        assert!(String::from_utf8_lossy(&output.stderr).contains("rustc --version failed"));
        assert!(!String::from_utf8_lossy(&output.stderr).contains("guard guard/rust"));
        std::fs::write(&rustc, "#!/bin/sh\nprintf '%s\\n' 'rustc changed-world'\n")
            .expect("changed tool");
        let output = guard.output().expect("guard");
        assert!(
            !output.status.success(),
            "changed broken tool cannot reuse proof"
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains("guard guard/rust"));
        std::fs::write(
            &rustc,
            format!(
                "#!/bin/sh\nprintf '%s' '{}'\n",
                version.replace('\'', "'\\''")
            ),
        )
        .expect("same version, different tool");
        let output = guard.output().expect("guard");
        assert!(
            !output.status.success(),
            "same version cannot hide changed tool"
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains("guard guard/rust"));
    }
}
