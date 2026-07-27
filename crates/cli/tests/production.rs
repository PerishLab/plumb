use std::path::Path;
use std::process::Command;

#[test]
fn boundary_debt_is_composite() {
    let root = std::env::temp_dir().join("plumb-production-boundary");
    let _ = std::fs::remove_dir_all(&root);
    for path in [
        "apps/web",
        "crates/api/src",
        "deploy",
        "charts/specimen/templates",
    ] {
        std::fs::create_dir_all(root.join(path)).unwrap();
    }
    write(
        &root,
        "apps/web/package.json",
        r#"{"dependencies":{"react":"19","vite":"7"}}"#,
    );
    write(
        &root,
        "crates/api/Cargo.toml",
        "[package]\nname = \"api\"\nversion = \"0.1.0\"\n",
    );
    write(&root, "crates/api/src/main.rs", "fn main() {}\n");
    write(
        &root,
        "deploy/web.Dockerfile",
        "FROM nginx\nENV API_UPSTREAM=api:3400\n",
    );
    write(
        &root,
        "charts/specimen/templates/ingress.yaml",
        "kind: Ingress\n- path: /\n  service:\n    name: specimen-web\n",
    );

    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["doctor", root.to_str().unwrap()])
        .output()
        .unwrap();
    std::fs::remove_dir_all(&root).unwrap();
    let out = String::from_utf8_lossy(&output.stdout);
    for message in [
        "web image does not run the emitted design runtime",
        "web image still owns public proxy dispatch",
        "chart ingress does not split /api and / between api and web",
    ] {
        assert!(out.contains(&format!("{message} [dispatch]")), "{out}");
    }
}

fn write(root: &Path, path: &str, text: &str) {
    std::fs::write(root.join(path), text).unwrap();
}
