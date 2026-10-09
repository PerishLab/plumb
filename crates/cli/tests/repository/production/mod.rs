mod lane;
mod preview;
mod qualification;
mod world;

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

const OCI: &str = "[release.oci]\nregistry = \"ghcr.io\"\nimage = \"perishlab/specimen\"\naccount = \"perishlab\"\n";
const WORKER: &str =
    "[release.cfworker]\naccount = \"perishlab\"\ndomain = \"specimen.perish.uk\"\n";

#[test]
fn container() {
    let root = specimen();
    let seat = "production has no api image seat [dispatch]";
    let bare = doctor(root.path());
    assert!(bare.contains(seat), "{bare}");

    write(root.path(), "Containerfile", "FROM scratch\n");
    let loose = doctor(root.path());
    assert!(loose.contains(seat), "{loose}");

    write(root.path(), "plumb.toml", OCI);
    let held = doctor(root.path());
    assert!(!held.contains(seat), "{held}");

    std::fs::remove_file(root.path().join("Containerfile")).unwrap();
    let empty = doctor(root.path());
    assert!(empty.contains(seat), "{empty}");
}

#[test]
fn worker() {
    let root = specimen();
    write(
        root.path(),
        "charts/specimen/templates/api.yaml",
        "kind: Deployment\nmetadata:\n  name: specimen-api\n",
    );
    write(
        root.path(),
        "charts/specimen/templates/ingress.yaml",
        "kind: Ingress\n- path: /api\n  service:\n    name: specimen-api\n",
    );
    let refused = [
        "production has no web image seat",
        "chart does not split api and web workloads",
        "chart ingress does not split /api and / between api and web",
    ];

    write(root.path(), "plumb.toml", OCI);
    let held = doctor(root.path());
    for message in refused {
        assert!(held.contains(&format!("{message} [dispatch]")), "{held}");
    }

    write(root.path(), "plumb.toml", &format!("{OCI}{WORKER}"));
    let placed = doctor(root.path());
    for message in refused {
        assert!(!placed.contains(message), "{placed}");
    }
    for message in [
        "web image does not run the emitted design runtime",
        "web image still owns public proxy dispatch",
        "beside the worker-placed web",
    ] {
        assert!(!placed.contains(message), "{placed}");
    }

    std::fs::remove_file(root.path().join("charts/specimen/templates/api.yaml")).unwrap();
    write(
        root.path(),
        "charts/specimen/templates/ingress.yaml",
        "kind: Ingress\n- path: /\n  service:\n    name: specimen-web\n",
    );
    let bare = doctor(root.path());
    for message in [
        "chart has no api workload beside the worker-placed web",
        "chart ingress does not route /api to api beside the worker-placed web",
    ] {
        assert!(bare.contains(&format!("{message} [dispatch]")), "{bare}");
    }
}

fn specimen() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    for path in ["apps/web", "crates/api/src", "charts/specimen/templates"] {
        std::fs::create_dir_all(root.path().join(path)).unwrap();
    }
    write(
        root.path(),
        "apps/web/package.json",
        r#"{"dependencies":{"svelte":"5","vite":"7"}}"#,
    );
    write(
        root.path(),
        "crates/api/Cargo.toml",
        "[package]\nname = \"api\"\nversion = \"0.1.0\"\n",
    );
    write(root.path(), "crates/api/src/main.rs", "fn main() {}\n");
    root
}

fn doctor(root: &Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .env("PLUMB_HOME", super::support::home().keep())
        .args(["doctor", root.to_str().expect("path should be utf8")])
        .output()
        .expect("plumb should run");
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn write(root: &Path, path: &str, text: &str) {
    std::fs::write(root.join(path), text).unwrap();
}
