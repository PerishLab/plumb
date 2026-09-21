mod authority;
mod catalog;
mod cloud;
mod depot;
mod identity;
mod registry;
mod retire;

fn product(_: &std::path::Path) -> Result<String, String> {
    unreachable!("registry state tests do not inspect repository shape")
}

use std::path::{Path, PathBuf};
use std::process::Command;

fn run(root: &Path) -> (bool, String) {
    let home = depot::support::depot(&[]);
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["doctor", root.to_str().expect("path should be utf8")])
        .env("PLUMB_HOME", home.path())
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
        "apps/web/src",
        "crates/api/src",
        "deploy",
        "charts/specimen/templates",
        ".forgejo/workflows",
    ] {
        std::fs::create_dir_all(root.join(path)).expect("fixture should be made");
    }
    std::fs::write(
        root.join("apps/web/package.json"),
        r#"{
  "name": "@specimen/web",
  "scripts": {
    "build": "vite build"
  },
  "dependencies": {
    "@perish/design": "0.2.1",
    "svelte": "5"
  },
  "devDependencies": {
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
fn complete() {
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
health_url = "http://127.0.0.1:{port}/api/health"
ready = { role = "api" }

[app]
name = "web"
port = 0
health_url = "http://127.0.0.1:{port}"
inherits_env = [{ name = "API_URL", from = "api.endpoint" }]
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
    let _router = Router::new().nest("/api", api);
}
"#,
    )
    .expect("api source should be written");
    std::fs::write(
        root.join("apps/web/vite.config.ts"),
        r#"
import { design } from "@perish/vite-plugin-design";
export default { plugins: [design()] };
"#,
    )
    .expect("vite config should be written");
    std::fs::write(
        root.join("apps/web/src/main.tsx"),
        r#"
import source from "virtual:perish/views";
import { Views } from "@perish/react-components";
const app = <Views source={source} />;
"#,
    )
    .expect("web entry should be written");
    std::fs::write(
        root.join("apps/web/tsconfig.json"),
        r#"{"compilerOptions":{"types":["vite/client","@perish/react-components/client"]}}"#,
    )
    .expect("web compiler should be written");
    std::fs::write(
        root.join(".forgejo/workflows/guard.yml"),
        "run: pnpm --filter @specimen/web build\n",
    )
    .expect("guard should be written");
    std::fs::write(root.join("deploy/api.Dockerfile"), "FROM scratch\n")
        .expect("api image should be written");
    std::fs::write(
        root.join("deploy/web.Dockerfile"),
        "FROM node\nCMD [\"node\", \"dist/.perish/server.mjs\", \"dist\"]\n",
    )
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
        r#"kind: Ingress
spec:
  rules:
    - http:
        paths:
          - path: /api
            backend:
              service:
                name: specimen-api
          - path: /
            backend:
              service:
                name: specimen-web
"#,
    )
    .expect("ingress should be written");

    let (_, out) = run(&root);
    assert!(!out.contains("[dispatch]"), "{out}");

    std::fs::write(
        root.join("sidecar.toml"),
        r#"
[[sidecars]]
name = "api"
command = "sh"
args = ["-c", "exec cargo run -p api"]
port = 0
env = { KEEL_LISTEN_PORT = "{port}" }
health_url = "http://127.0.0.1:{port}/api/health"
ready = { role = "api" }

[app]
name = "web"
port = 0
health_url = "http://127.0.0.1:{port}"
inherits_env = [{ name = "API_URL", from = "api.endpoint" }]
"#,
    )
    .expect("sidecar should be written");
    std::fs::write(
        root.join("crates/api/src/main.rs"),
        r#"
fn main() {
    let sidecar_stamp = "";
    eprintln!("{}", serde_json::json!({"role": "api", "endpoint": sidecar_stamp}));
    let _router = Router::new().nest("/api", api);
}
"#,
    )
    .expect("api source should be written");
    let (_, out) = run(&root);
    assert!(!out.contains("api does not consume SIDECAR_PORT"), "{out}");

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
    let (_, out) = run(&root);
    for line in [
        "sidecar api health_url must target {port}/api/health",
        "sidecar web must inherit api.endpoint as API_URL",
    ] {
        assert!(out.contains(&format!("{line} [dispatch]")), "{out}");
    }
    std::fs::remove_dir_all(&root).expect("fixture should be swept");
}

#[test]
fn incomplete() {
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
        r#"
const port = process.env.SIDECAR_PORT;
const api = process.env.API_URL;
export default { server: { port, proxy: { "/api": api } } };
"#,
    )
    .expect("vite config should be written");

    let (success, out) = run(&root);
    std::fs::remove_dir_all(&root).expect("fixture should be swept");
    assert!(!success, "{out}");
    for line in [
        "vite does not activate the design plugin",
        "vite manually consumes sidecar dispatch environment",
        "web does not load the virtual views manifest",
        "web does not render the views manifest",
        "web does not declare the virtual views module type",
    ] {
        assert!(out.contains(&format!("{line} [web]")), "{out}");
    }
    for line in [
        "web/api pair has no sidecar.toml",
        "api does not consume SIDECAR_PORT",
        "api does not accept --sidecar-stamp",
        "api does not emit api endpoint readiness",
        "api does not mount the /api namespace",
        "production has no api image seat",
        "production has no web image seat",
        "chart does not split api and web workloads",
        "chart ingress does not split /api and / between api and web",
        "Cargo and chart do not share one version train",
    ] {
        assert!(out.contains(&format!("{line} [dispatch]")), "{out}");
    }
}

#[test]
fn malformed() {
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
