use super::{Repo, cache, support};

#[test]
fn declaration() {
    let fixture = cache::fixture();
    let root = fixture.path();
    let home = support::depot(&[]);
    let first = cache::run(root, home.path());
    cache::success(&first);
    std::fs::write(root.join("plumb.toml"), "invalid [ unstaged").expect("working policy");
    let unchanged = cache::run(root, home.path());
    cache::success(&unchanged);
    assert_eq!(first.stdout, unchanged.stdout);
    assert!(!String::from_utf8_lossy(&unchanged.stderr).contains("guard guard/rust"));
    Repo::git(root, &["add", "plumb.toml"]);
    let invalid = cache::run(root, home.path());
    assert!(!invalid.status.success());
    let diagnostic = String::from_utf8_lossy(&invalid.stderr);
    assert!(
        diagnostic.contains("cannot parse staged plumb.toml"),
        "{diagnostic}"
    );
    assert!(!diagnostic.contains("guard guard/rust"), "{diagnostic}");
}

#[cfg(unix)]
struct Web {
    root: tempfile::TempDir,
    home: tempfile::TempDir,
    tools: tempfile::TempDir,
}

#[cfg(unix)]
impl Web {
    fn new() -> Self {
        use std::os::unix::fs::PermissionsExt as _;
        let held = Self {
            root: cache::fixture(),
            home: support::depot(&[]),
            tools: tempfile::tempdir().expect("tools"),
        };
        let root = held.root.path();
        std::fs::write(
            root.join("plumb.toml"),
            "[workflow.hash.guard]\nweb=['apps','biome.json']\n",
        )
        .expect("web policy");
        std::fs::create_dir_all(root.join("apps/web")).expect("app");
        std::fs::write(
            root.join("apps/web/package.json"),
            r#"{"name":"staged","scripts":{"build":"held"}}"#,
        )
        .expect("app manifest");
        std::fs::write(root.join("biome.json"), "{}").expect("biome");
        Repo::git(root, &["add", "."]);
        for name in ["node", "pnpm", "corepack"] {
            let path = held.tools.path().join(name);
            let body = format!(
                "#!/bin/sh\nif [ \"$1\" = --version ]; then echo fixture; exit 0; fi\nprintf '%s\\n' \"$*\" >> '{}'\n",
                held.tools.path().join("calls").display()
            );
            std::fs::write(&path, body).expect("stub");
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
                .expect("executable");
        }
        held
    }

    fn run(&self) -> std::process::Output {
        let mut paths = vec![self.tools.path().to_path_buf()];
        paths.extend(std::env::split_paths(
            &std::env::var_os("PATH").expect("PATH"),
        ));
        std::process::Command::new(env!("CARGO_BIN_EXE_plumb"))
            .args(["guard", ".", "--json"])
            .current_dir(self.root.path())
            .env("PLUMB_HOME", self.home.path())
            .env("PATH", std::env::join_paths(paths).expect("PATH"))
            .output()
            .expect("guard")
    }
}

#[test]
#[cfg(unix)]
fn working() {
    let fixture = Web::new();
    let root = fixture.root.path();
    std::fs::write(
        root.join("apps/web/package.json"),
        r#"{"name":"unstaged","scripts":{"build":"held"}}"#,
    )
    .expect("working manifest");
    std::fs::remove_file(root.join("biome.json")).expect("unstaged deletion");
    let first = fixture.run();
    cache::success(&first);
    let calls = std::fs::read_to_string(fixture.tools.path().join("calls")).expect("calls");
    assert!(calls.contains("biome ci ."), "{calls}");
    assert!(calls.contains("--filter staged build"), "{calls}");
    assert!(!calls.contains("unstaged"), "{calls}");
    Repo::git(root, &["add", "apps/web/package.json", "biome.json"]);
    let changed = fixture.run();
    cache::success(&changed);
    assert_ne!(first.stdout, changed.stdout);
    let after = std::fs::read_to_string(fixture.tools.path().join("calls")).expect("calls");
    let added = after.strip_prefix(&calls).expect("new calls");
    assert!(added.contains("--filter unstaged build"), "{added}");
    assert!(!added.contains("biome"), "{added}");
    std::fs::write(root.join("biome.json"), "{}").expect("untracked biome");
    std::fs::create_dir_all(root.join("apps/extra")).expect("untracked app");
    std::fs::write(
        root.join("apps/extra/package.json"),
        "invalid untracked manifest",
    )
    .expect("untracked manifest");
    cache::success(&fixture.run());
    assert_eq!(
        after,
        std::fs::read_to_string(fixture.tools.path().join("calls")).expect("calls")
    );
}

#[test]
#[cfg(unix)]
fn malformed() {
    let fixture = Web::new();
    let root = fixture.root.path();
    std::fs::write(
        root.join("apps/web/package.json"),
        "invalid private content",
    )
    .expect("invalid manifest");
    Repo::git(root, &["add", "apps/web/package.json"]);
    let output = fixture.run();
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        error.contains("invalid staged package manifest apps/web/package.json"),
        "{error}"
    );
    assert!(!error.contains("invalid private content"));
    assert!(!fixture.tools.path().join("calls").exists());
}
