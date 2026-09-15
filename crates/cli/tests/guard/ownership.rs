use std::path::Path;

fn fixture() -> tempfile::TempDir {
    let root = super::super::fixture();
    std::fs::write(
        root.path().join("plumb.toml"),
        "[[layout.seat]]\npath='policy'\ndepot='rules'\n[[layout.seat]]\npath='profiles'\ndepot='profiles'\n[[layout.seat]]\npath='payload'\ndepot='assets'\n",
    )
    .unwrap();
    for path in ["policy", "profiles", "payload"] {
        std::fs::create_dir(root.path().join(path)).unwrap();
    }
    root
}

fn findings(root: &Path, seat: &Path) -> Vec<String> {
    assert!(
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(root)
            .status()
            .unwrap()
            .success()
    );
    let output = super::run(root, seat);
    assert!(output.contains("the held depot"), "{output}");
    output
        .lines()
        .filter(|line| line.contains("the held depot"))
        .map(str::to_string)
        .collect()
}

#[test]
fn policy() {
    let root = fixture();
    let seat = super::super::super::support::depot(&[]);
    std::fs::write(root.path().join("policy/catalog.toml"), "source policy").unwrap();
    std::fs::write(root.path().join("profiles/source.toml"), "source profile").unwrap();
    let found = findings(root.path(), seat.path());
    assert!(
        found
            .iter()
            .all(|line| !line.contains("rules/") && !line.contains("profiles/")),
        "{found:?}"
    );
    assert!(
        found.iter().any(|line| line.contains("assets/")),
        "{found:?}"
    );
}

#[test]
fn resources() {
    let root = fixture();
    let seat = super::super::super::support::depot(&[("assets/probe.txt", "published")]);
    std::fs::write(root.path().join("payload/probe.txt"), "changed").unwrap();
    std::fs::write(root.path().join("payload/new.txt"), "unpublished").unwrap();
    let found = findings(root.path(), seat.path());
    assert!(
        found
            .iter()
            .any(|line| line.contains("different assets/probe.txt")),
        "{found:?}"
    );
    assert!(
        found
            .iter()
            .any(|line| line.contains("carries no assets/new.txt")),
        "{found:?}"
    );
    std::fs::write(root.path().join("payload/probe.txt"), "published").unwrap();
    let found = findings(root.path(), seat.path());
    assert!(
        found.iter().all(|line| !line.contains("assets/probe.txt")),
        "{found:?}"
    );
}
