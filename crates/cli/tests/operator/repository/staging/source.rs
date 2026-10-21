use super::{Repo, cache, support};
use std::path::{Path, PathBuf};

pub(super) fn seat(home: &Path) -> PathBuf {
    std::fs::read_dir(home.join("tmp"))
        .expect("staging home")
        .map(|entry| entry.expect("source entry").path())
        .find(|path| path.is_dir())
        .expect("retained source")
}

#[test]
fn fresh() {
    let fixture = cache::fixture();
    let root = fixture.path();
    let home = support::home();
    std::fs::write(root.join("policy"), "first\n").unwrap();
    std::fs::write(
        root.join("plumb.toml"),
        "[workflow.hash.guard]\nrust = ['Cargo.toml', 'Cargo.lock', 'src', 'policy']\n",
    )
    .unwrap();
    Repo::git(root, &["add", "policy", "plumb.toml"]);
    cache::success(&cache::run(root, home.path()));
    let source = seat(home.path());
    let stamp = std::fs::metadata(source.join("src/lib.rs"))
        .unwrap()
        .modified()
        .unwrap();
    std::fs::write(root.join("policy"), "second\n").unwrap();
    Repo::git(root, &["add", "policy"]);
    let next = cache::run(root, home.path());
    cache::success(&next);
    let log = String::from_utf8_lossy(&next.stderr);
    assert!(
        log.contains("guard guard/rust"),
        "a changed action must run: {log}"
    );
    assert!(
        !log.contains("Checking probe"),
        "unchanged Cargo inputs must stay fresh: {log}"
    );
    assert_eq!(seat(home.path()), source);
    assert_eq!(
        std::fs::metadata(source.join("src/lib.rs"))
            .unwrap()
            .modified()
            .unwrap(),
        stamp
    );
    assert_eq!(
        std::fs::read_to_string(source.join("policy")).unwrap(),
        "second\n"
    );
}

#[test]
fn invalidation() {
    let fixture = cache::fixture();
    let root = fixture.path();
    let home = support::home();
    std::fs::write(
        root.join("src/api.rs"),
        "pub fn answer() -> u8 {\n    42\n}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/lib.rs"),
        "pub mod api;\npub fn answer() -> u8 {\n    api::answer()\n}\n",
    )
    .unwrap();
    Repo::git(root, &["add", "src"]);
    cache::success(&cache::run(root, home.path()));
    std::fs::write(
        root.join("src/api.rs"),
        "pub fn different() -> u8 {\n    43\n}\n",
    )
    .unwrap();
    Repo::git(root, &["add", "src/api.rs"]);
    let failed = cache::run(root, home.path());
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("cannot find function `answer`"));
    std::fs::write(
        root.join("src/lib.rs"),
        "pub mod api;\npub fn answer() -> u8 {\n    api::different()\n}\n",
    )
    .unwrap();
    Repo::git(root, &["add", "src/lib.rs"]);
    std::fs::write(root.join("src/api.rs"), "uncommitted invalid source\n").unwrap();
    cache::success(&cache::run(root, home.path()));
    assert_eq!(
        std::fs::read_to_string(root.join("src/api.rs")).unwrap(),
        "uncommitted invalid source\n"
    );
}

#[test]
fn features() {
    let fixture = cache::fixture();
    let root = fixture.path();
    let home = support::home();
    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        format!("{manifest}\n[features]\ndefault = ['active']\nactive = []\n"),
    )
    .unwrap();
    std::fs::write(root.join("src/lib.rs"), "#[cfg(feature = \"active\")]\npub fn selected() -> u8 {\n    42\n}\npub fn answer() -> u8 {\n    selected()\n}\n").unwrap();
    Repo::git(root, &["add", "Cargo.toml", "src"]);
    cache::success(&cache::run(root, home.path()));
    std::fs::write(
        root.join("Cargo.toml"),
        format!("{manifest}\n[features]\ndefault = []\nactive = []\n"),
    )
    .unwrap();
    Repo::git(root, &["add", "Cargo.toml"]);
    let refused = cache::run(root, home.path());
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("cannot find function `selected`"));
}

#[test]
fn mutation() {
    let fixture = cache::fixture();
    let root = fixture.path();
    let home = support::home();
    std::fs::write(root.join("policy"), "original\n").unwrap();
    std::fs::write(root.join("build.rs"), "fn main() {\n    std::fs::write(\"policy\", \"changed\\n\").unwrap();\n    assert!(std::process::Command::new(\"git\").args([\"add\", \"policy\"]).status().unwrap().success());\n}\n").unwrap();
    std::fs::write(
        root.join("plumb.toml"),
        "[workflow.hash.guard]\nrust = ['Cargo.toml', 'Cargo.lock', 'src', 'build.rs', 'policy']\n",
    )
    .unwrap();
    let format = std::process::Command::new("cargo")
        .args(["fmt", "--all"])
        .current_dir(root)
        .output()
        .unwrap();
    cache::success(&format);
    Repo::git(root, &["add", "build.rs", "policy", "plumb.toml"]);
    let tree = Repo::git(root, &["write-tree"]).stdout;
    for _ in 0..2 {
        let refused = cache::run(root, home.path());
        assert!(!refused.status.success());
        assert!(
            String::from_utf8_lossy(&refused.stderr)
                .contains("changed a tracked verification input"),
            "{}",
            String::from_utf8_lossy(&refused.stderr)
        );
    }
    assert_eq!(Repo::git(root, &["write-tree"]).stdout, tree);
    assert_eq!(
        std::fs::read_to_string(root.join("policy")).unwrap(),
        "original\n"
    );
}
