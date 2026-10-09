use super::run;
use std::path::Path;

#[test]
fn loader() {
    let root = std::env::temp_dir().join("plumb-loader");
    std::fs::create_dir_all(root.join("crates/cli")).expect("fixture should be made");
    std::fs::create_dir_all(root.join("tools/probe")).expect("fixture should be made");
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/cli\", \"tools/*\"]\n\n[workspace.dependencies]\nkept = \"1\"\n",
    )
    .expect("workspace should be written");
    std::fs::write(
        root.join("tools/probe/Cargo.toml"),
        "[package]\nname = \"probe\"\n\n[target.'cfg(unix)'.dev-dependencies]\ndotenvy = \"0.15\"\n",
    )
    .expect("member should be written");
    std::fs::write(
        root.join("crates/cli/Cargo.toml"),
        "[package]\nname = \"cli\"\n\n[dependencies]\nenv = { package = \"dotenvy\", version = \"0.15\" }\n",
    )
    .expect("manifest should be written");
    let out = run(&root);
    std::fs::remove_dir_all(&root).expect("fixture should be swept");
    let manifest = Path::new("crates").join("cli").join("Cargo.toml");
    let line = format!(
        "{} depends on .env loader dotenvy; declare settings through plumb Cascade",
        manifest.display()
    );
    assert!(out.contains(&line), "{out}");
    let member = Path::new("tools").join("probe").join("Cargo.toml");
    assert!(
        out.contains(&format!(
            "{} depends on .env loader dotenvy",
            member.display()
        )),
        "{out}"
    );
    assert!(!out.contains("depends on .env loader kept"), "{out}");
}
