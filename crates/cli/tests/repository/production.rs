use std::path::Path;
use std::process::Command;

#[test]
fn boundary() {
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
        r#"{"dependencies":{"svelte":"5","vite":"7"}}"#,
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

    let out = doctor(&root);
    std::fs::remove_dir_all(&root).unwrap();
    for message in [
        "web image does not run the emitted design runtime",
        "web image still owns public proxy dispatch",
        "chart ingress does not split /api and / between api and web",
    ] {
        assert!(out.contains(&format!("{message} [dispatch]")), "{out}");
    }
}

#[test]
fn sites() {
    let root = std::env::temp_dir().join("plumb-siteless");
    std::fs::create_dir_all(root.join("apps/web")).expect("fixture should be made");
    let bare = doctor(&root);
    assert!(!bare.contains("declares a site"), "{bare}");

    std::fs::write(root.join("apps/web/wrangler.jsonc"), "{}").expect("config should be written");
    let held = doctor(&root);
    std::fs::remove_dir_all(&root).expect("fixture should be swept");
    assert!(!held.contains("declares a site"), "{held}");
    assert!(held.contains("sites     web"), "{held}");
}

fn doctor(root: &Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .env("PLUMB_HOME", super::support::seat())
        .args(["doctor", root.to_str().expect("path should be utf8")])
        .output()
        .expect("plumb should run");
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn write(root: &Path, path: &str, text: &str) {
    std::fs::write(root.join(path), text).unwrap();
}
