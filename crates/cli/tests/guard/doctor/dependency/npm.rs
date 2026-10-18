use std::path::Path;
use std::process::{Command, Output};

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().into()
}

fn fixture() -> tempfile::TempDir {
    let root = super::super::fixture();
    let declaration = std::fs::read_to_string(root.path().join("plumb.toml")).unwrap();
    std::fs::write(
        root.path().join("plumb.toml"),
        format!("{declaration}\n[[layout.file]]\nname=['package.json','pnpm-lock.yaml']\n"),
    )
    .unwrap();
    std::fs::write(
        root.path().join("package.json"),
        r#"{"dependencies":{"@perishlab/design":"0"}}"#,
    )
    .unwrap();
    std::fs::write(root.path().join("pnpm-lock.yaml"), "lockfileVersion: '9.0'\nimporters:\n  .:\n    dependencies:\n      '@perishlab/design':\n        specifier: '0'\n        version: 0.3.14(svelte@5.0.0)\npackages:\n  '@perishlab/design@0.3.14': {}\n").unwrap();
    git(root.path(), &["add", "."]);
    root
}

fn observed(root: &Path, context: &str, latest: &str) -> serde_json::Value {
    serde_json::json!({"schema":1,"resolution":{"context":context,"tree":git(root, &["write-tree"]),"packages":[{"ecosystem":"npm","name":"@perishlab/design","version":latest}]}})
}

fn observing(observation: &serde_json::Value) -> impl FnOnce(&mut Command) + '_ {
    |command| {
        command.env("PLUMB_PACKAGES_OBSERVATION", observation.to_string());
    }
}

fn run(root: &Path, bind: impl FnOnce(&mut Command)) -> Output {
    let home = crate::support::home();
    let mut command = crate::support::plumb();
    command
        .args(["doctor", root.to_str().unwrap()])
        .env_remove("PLUMB_RELEASE_VERSION")
        .env_remove("WHARF_PACKAGES_TOKEN")
        .env_remove("PLUMB_PACKAGES_OBSERVATION")
        .env(
            "NPM_CONFIG_USERCONFIG",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env("PLUMB_HOME", home.path());
    bind(&mut command);
    command.output().unwrap()
}

#[test]
fn currency() {
    let root = fixture();
    let current = run(
        root.path(),
        observing(&observed(root.path(), "ci-latest", "0.3.14")),
    );
    let said = String::from_utf8_lossy(&current.stdout);
    assert!(current.status.success(), "{said}");
    assert!(
        said.contains("npm @perishlab/design 0 -> 0.3.14 (latest 0.3.14)"),
        "{said}"
    );
    let stale = run(
        root.path(),
        observing(&observed(root.path(), "ci-latest", "0.3.15")),
    );
    let said = String::from_utf8_lossy(&stale.stdout);
    assert!(!stale.status.success(), "{said}");
    assert!(
        said.contains("0.3.14") && said.contains("0.3.15") && said.contains("out of true"),
        "{said}"
    );
}

#[test]
fn lagging() {
    let root = fixture();
    let output = run(
        root.path(),
        observing(&observed(root.path(), "local-locked", "0.3.15")),
    );
    let said = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{said}");
    assert!(
        said.contains("noted: npm @perishlab/design is locked at 0.3.14, stable latest is 0.3.15; the organization Guard late-binds it [package.json]"),
        "{said}"
    );
    assert!(!said.contains("out of true:"), "{said}");
    let mut observation = observed(root.path(), "local-locked", "0.3.15");
    observation["schema"] = 99.into();
    let output = run(root.path(), observing(&observation));
    let said = String::from_utf8_lossy(&output.stdout);
    assert!(!output.status.success(), "{said}");
    assert!(
        said.contains("blind: package observation schema or context differs"),
        "{said}"
    );
}

#[cfg(unix)]
#[test]
fn standalone() {
    let root = fixture();
    let tools = tempfile::tempdir().unwrap();
    crate::support::executable(
        &tools.path().join("pnpm"),
        "#!/bin/sh\nif [ \"$1\" != --config.ignore-pnpmfile=true ] || [ \"$2\" != view ]; then exit 91; fi\ncat \"$(dirname \"$0\")/latest\"\n",
    );
    let mut paths = vec![tools.path().to_path_buf()];
    paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    let path = std::env::join_paths(paths).unwrap();
    let alone = || {
        run(root.path(), |command| {
            command.env("PATH", &path);
        })
    };
    std::fs::write(tools.path().join("latest"), "\"0.3.15\"").unwrap();
    let output = alone();
    let said = String::from_utf8_lossy(&output.stdout);
    assert!(!output.status.success(), "{said}");
    assert!(
        said.contains(
            "out of true: npm @perishlab/design resolves to 0.3.14, stable latest is 0.3.15"
        ),
        "{said}"
    );
    assert!(!said.contains("noted: npm"), "{said}");
    std::fs::remove_file(tools.path().join("latest")).unwrap();
    let output = alone();
    let said = String::from_utf8_lossy(&output.stdout);
    assert!(!output.status.success(), "{said}");
    assert!(
        said.contains("blind:") && !said.contains("noted: npm"),
        "{said}"
    );
}

#[test]
fn blind() {
    let root = fixture();
    let mut observation = observed(root.path(), "ci-latest", "0.3.14");
    observation["schema"] = 99.into();
    let output = run(root.path(), observing(&observation));
    let said = String::from_utf8_lossy(&output.stdout);
    assert!(!output.status.success(), "{said}");
    assert!(
        said.contains("package observation schema or context differs"),
        "{said}"
    );
    assert!(!said.contains("true to the skeleton"), "{said}");
    let observation = observed(root.path(), "ci-latest", "0.3.14");
    std::fs::write(root.path().join("changed"), "different tree").unwrap();
    git(root.path(), &["add", "changed"]);
    let output = run(root.path(), observing(&observation));
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("package observation tree differs"));
}

#[test]
fn datum() {
    let root = fixture();
    git(
        root.path(),
        &["symbolic-ref", "HEAD", "refs/heads/release/v1.2.0"],
    );
    let seat = root.path().join(".plumb/releases/v1.2.0");
    std::fs::create_dir_all(&seat).unwrap();
    std::fs::write(seat.join("datum.toml"), "schema=1\nversion='v1.2.0'\n[[answer]]\necosystem='npm'\nname='@perishlab/design'\nlatest='0.3.14'\n").unwrap();
    let output = run(
        root.path(),
        observing(&observed(root.path(), "ci-latest", "0.3.15")),
    );
    let said = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{said}");
    assert!(said.contains("latest 0.3.14"), "{said}");
    assert!(!said.contains("latest 0.3.15"), "{said}");
}
