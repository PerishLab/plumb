use std::path::{Path, PathBuf};
use std::process::Command;

fn run(root: &Path) -> (bool, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["doctor", root.to_str().expect("path should be utf8")])
        .output()
        .expect("plumb should run");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
    )
}

fn fixture(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(name);
    if root.exists() {
        std::fs::remove_dir_all(&root).expect("old fixture should be swept");
    }
    for path in [
        "apps/web",
        "crates/api/src",
        "deploy",
        "charts/specimen/templates",
    ] {
        std::fs::create_dir_all(root.join(path)).expect("fixture should be made");
    }
    std::fs::write(
        root.join("apps/web/package.json"),
        r#"{
  "dependencies": {
    "@perish/react-components": "0.1.0",
    "react": "19"
  },
  "devDependencies": {
    "@perish/vite-plugin-design": "0.1.0",
    "vite": "7"
  }
}"#,
    )
    .expect("web manifest should be written");
    std::fs::write(
        root.join("crates/api/Cargo.toml"),
        "[package]\nname = \"api\"\nversion = \"0.1.0\"\n",
    )
    .expect("api manifest should be written");
    root
}

#[test]
fn complete_pair_is_true() {
    let root = fixture("plumb-dispatch-complete");
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace.package]\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("workspace manifest should be written");
    std::fs::write(root.join(".gitignore"), "target/\n").expect("ignore should be written");
    std::fs::write(
        root.join("sidecar.toml"),
        r#"
[[sidecars]]
name = "api"
port = 0
health_url = "http://127.0.0.1:{port}/health"
ready = { role = "api" }

[app]
name = "web"
port = 0
health_url = "http://127.0.0.1:{port}"
inherits_env = [{ name = "SPECIMEN_API", from = "api.endpoint" }]
"#,
    )
    .expect("sidecar should be written");
    std::fs::write(
        root.join("crates/api/src/main.rs"),
        r#"
fn main() {
    let _port = std::env::var("SIDECAR_PORT");
    let sidecar_stamp = "";
    eprintln!("{}", serde_json::json!({"role": "api", "endpoint": sidecar_stamp}));
}
"#,
    )
    .expect("api source should be written");
    std::fs::write(
        root.join("apps/web/vite.config.ts"),
        r#"
import { design } from "@perish/vite-plugin-design";
const port = process.env.SIDECAR_PORT;
const api = process.env.SPECIMEN_API;
export default { plugins: [design()], server: { port, proxy: { "/": api } } };
"#,
    )
    .expect("vite config should be written");
    std::fs::write(root.join("deploy/api.Dockerfile"), "FROM scratch\n")
        .expect("api image should be written");
    std::fs::write(root.join("deploy/web.Dockerfile"), "FROM scratch\n")
        .expect("web image should be written");
    std::fs::write(
        root.join("charts/specimen/Chart.yaml"),
        "version: 0.1.0\nappVersion: \"0.1.0\"\n",
    )
    .expect("chart should be written");
    std::fs::write(
        root.join("charts/specimen/templates/api.yaml"),
        "kind: Deployment\nmetadata:\n  name: specimen-api\n",
    )
    .expect("api workload should be written");
    std::fs::write(
        root.join("charts/specimen/templates/web.yaml"),
        "kind: Deployment\nmetadata:\n  name: specimen-web\n",
    )
    .expect("web workload should be written");
    std::fs::write(
        root.join("charts/specimen/templates/ingress.yaml"),
        "kind: Ingress\nbackend:\n  service:\n    name: specimen-web\n",
    )
    .expect("ingress should be written");

    let (_, out) = run(&root);
    std::fs::remove_dir_all(&root).expect("fixture should be swept");
    assert!(!out.contains("[dispatch]"), "{out}");
}

#[test]
fn incomplete_pair_reports_the_composite_debt() {
    let root = fixture("plumb-dispatch-incomplete");
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace.package]\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("workspace manifest should be written");
    std::fs::write(root.join(".gitignore"), "target/\n").expect("ignore should be written");
    std::fs::write(root.join("crates/api/src/main.rs"), "fn main() {}\n")
        .expect("api source should be written");
    std::fs::write(
        root.join("apps/web/vite.config.ts"),
        "export default { server: { port: 5173 } };\n",
    )
    .expect("vite config should be written");

    let (success, out) = run(&root);
    std::fs::remove_dir_all(&root).expect("fixture should be swept");
    assert!(!success, "{out}");
    for line in [
        "web/api pair has no sidecar.toml",
        "api does not consume SIDECAR_PORT",
        "api does not accept --sidecar-stamp",
        "api does not emit api endpoint readiness",
        "vite does not consume SIDECAR_PORT",
        "vite does not activate the design plugin",
        "production has no api image seat",
        "production has no web image seat",
        "chart does not split api and web workloads",
        "chart ingress does not target web",
        "Cargo and chart do not share one version train",
    ] {
        assert!(out.contains(&format!("{line} [dispatch]")), "{out}");
    }
}

#[test]
fn malformed_sidecar_is_blind() {
    let root = fixture("plumb-dispatch-blind");
    std::fs::write(root.join("sidecar.toml"), "[[sidecars]\n").expect("sidecar should be written");
    std::fs::write(root.join("crates/api/src/main.rs"), "fn main() {}\n")
        .expect("api source should be written");

    let (_, out) = run(&root);
    std::fs::remove_dir_all(&root).expect("fixture should be swept");
    assert!(out.contains("cannot read sidecar.toml"), "{out}");
    assert!(out.contains("blind:"), "{out}");
    assert!(out.contains("[dispatch]"), "{out}");
}
