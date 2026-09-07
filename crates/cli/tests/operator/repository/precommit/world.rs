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
    assert!(action.find(".checks()?").expect("current actions") < reuse);
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
#[cfg(unix)]
fn unread() {
    use std::os::unix::fs::PermissionsExt as _;
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
    }
}
