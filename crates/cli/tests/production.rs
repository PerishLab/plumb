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
    std::fs::create_dir_all(root.join(".runseal/wrappers")).expect("fixture should be made");
    let bare = doctor(&root);
    assert!(!bare.contains("declares a site"), "{bare}");

    std::fs::write(root.join("apps/web/wrangler.jsonc"), "{}").expect("config should be written");
    let held = doctor(&root);
    assert!(
        held.contains("web declares a site without a ship wrapper"),
        "{held}"
    );
    assert!(
        held.contains("web declares a site without a deploy lane"),
        "{held}"
    );

    std::fs::write(root.join(".runseal/wrappers/ship.ts"), "").expect("wrapper should be written");
    std::fs::create_dir_all(root.join(".forgejo/workflows")).expect("fixture should be made");
    std::fs::write(root.join(".forgejo/workflows/deploy.yml"), "").expect("lane should be written");
    let paired = doctor(&root);
    std::fs::remove_dir_all(&root).expect("fixture should be swept");
    assert!(!paired.contains("declares a site"), "{paired}");
    assert!(paired.contains("sites     web"), "{paired}");
}

fn doctor(root: &Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["doctor", root.to_str().expect("path should be utf8")])
        .output()
        .expect("plumb should run");
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn write(root: &Path, path: &str, text: &str) {
    std::fs::write(root.join(path), text).unwrap();
}
