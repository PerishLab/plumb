use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn unix() {
    let fixture = seat();
    let script = script("migration.sh");
    let output = Command::new("sh")
        .arg(script)
        .current_dir(fixture.path())
        .output()
        .expect("migration");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    migrated(fixture.path());
}

#[test]
fn windows() {
    if Command::new("pwsh").arg("--version").output().is_err() {
        return;
    }
    let fixture = seat();
    let script = script("migration.ps1");
    let output = Command::new("pwsh")
        .args(["-NoProfile", "-File"])
        .arg(script)
        .current_dir(fixture.path())
        .output()
        .expect("migration");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    migrated(fixture.path());
}

fn seat() -> tempfile::TempDir {
    let fixture = tempfile::tempdir().expect("fixture");
    let root = fixture.path();
    std::fs::create_dir_all(root.join("skills/tool")).expect("skill seat");
    for leaf in ["SKILL.md", "PATHS.md", "SCENARIOS.md"] {
        std::fs::write(root.join("skills/tool").join(leaf), "brief\n").expect("brief");
    }
    std::fs::write(
        root.join("plumb.toml"),
        "[[lock]]\nname = \"skill\"\npaths = [\"skills/tool\"]\nversion = \"1.0.0\"\nhash = \"held\"\n\n[skill]\nstrategy = \"brief\"\n\n[release]\nproduct = \"tool\"\n",
    )
    .expect("manifest");
    fixture
}

fn migrated(root: &Path) {
    let text = std::fs::read_to_string(root.join("plumb.toml")).expect("manifest");
    assert!(!text.contains("[[lock]]"), "{text}");
    assert!(!text.contains("[skill]"), "{text}");
    assert!(text.contains("[release]"), "{text}");
    assert_eq!(text.matches("[[document]]").count(), 2, "{text}");
    assert!(text.contains("strategy = \"agent\""), "{text}");
    assert!(text.contains("strategy = \"brief\""), "{text}");
    assert!(text.contains("name = \"tool\""), "{text}");
    assert_eq!(text.matches("seal = \"\"").count(), 4, "{text}");
}

fn script(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/migration-v0.18.19")
        .join(name)
}
