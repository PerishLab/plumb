use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn seat(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("plumb-changelog-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("seat");
    fs::write(
        path.join("Cargo.toml"),
        "[workspace.package]\nversion = \"1.2.3\"\n",
    )
    .expect("cargo");
    path
}

fn logged(workspace: &Path, tongue: &str, leaf: &str, text: &str) {
    let home = workspace.join("docs/CHANGELOG/v1.2.3").join(tongue);
    fs::create_dir_all(&home).expect("home");
    fs::write(home.join(leaf), text).expect("leaf");
}

fn graded(workspace: &Path) -> (String, bool) {
    marked(workspace, &[])
}

fn marked(workspace: &Path, extra: &[&str]) -> (String, bool) {
    let mut args = vec!["changelog", workspace.to_str().expect("path")];
    args.extend_from_slice(extra);
    let out = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(&args)
        .output()
        .expect("run");
    let shown = String::from_utf8_lossy(&out.stdout).to_string();
    (shown, out.status.success())
}

#[test]
fn changelog() {
    let root = seat("changelog");
    let (bare, held) = graded(&root);
    assert!(!held, "{bare}");
    assert!(bare.contains("en/INDEX.md is missing"), "{bare}");
    assert!(bare.contains("zh/MIGRATION.md is missing"), "{bare}");

    for tongue in ["en", "zh"] {
        for leaf in ["INDEX.md", "MIGRATION.md"] {
            logged(&root, tongue, leaf, "written\n");
        }
    }
    let (full, done) = graded(&root);
    assert!(done, "{full}");
    assert!(full.contains("v1.2.3 is documented in en and zh"), "{full}");

    logged(&root, "zh", "MIGRATION.md", "   \n");
    let (blank, refused) = graded(&root);
    let _ = fs::remove_dir_all(&root);
    assert!(!refused, "{blank}");
    assert!(blank.contains("zh/MIGRATION.md is empty"), "{blank}");
}

#[test]
fn blank() {
    let workspace = seat("blank");
    for tongue in ["en", "zh"] {
        for leaf in ["INDEX.md", "MIGRATION.md"] {
            logged(&workspace, tongue, leaf, "written\n");
        }
    }
    let (empty, held) = marked(&workspace, &["--version", ""]);
    let (spaced, worn) = marked(&workspace, &["--version", "  "]);
    let _ = fs::remove_dir_all(&workspace);
    assert!(held, "{empty}");
    assert!(empty.contains("v1.2.3 is documented"), "{empty}");
    assert!(worn, "{spaced}");
    assert!(spaced.contains("v1.2.3 is documented"), "{spaced}");
}
