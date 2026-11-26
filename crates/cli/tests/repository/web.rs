use std::path::Path;
use std::process::Command;

fn run(root: &Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .env("PLUMB_HOME", super::support::home().keep())
        .args(["doctor", root.to_str().expect("path should be utf8")])
        .output()
        .expect("plumb should run");
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn fixture(name: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let scratch = tempfile::tempdir().expect("fixture parent should be made");
    let root = scratch.path().join(name);
    for path in [
        "apps/web/src/views",
        "apps/web/src/lib/components",
        "apps/web/src/lib/hooks",
        ".runseal/resources",
        ".forgejo/workflows",
    ] {
        std::fs::create_dir_all(root.join(path)).expect("fixture should be made");
    }
    std::fs::write(
        root.join("apps/web/package.json"),
        r#"{"name":"@specimen/web","scripts":{"build":"vite build"},"dependencies":{"@perishlab/design":"0.2.1","svelte":"5","vite":"8"},"devDependencies":{}}"#,
    )
    .expect("manifest should be written");
    std::fs::write(
        root.join("apps/web/vite.config.ts"),
        "import { design } from '@perishlab/design/vite'; export default { plugins: [design()] };",
    )
    .expect("vite config should be written");
    std::fs::write(
        root.join("apps/web/tsconfig.json"),
        r#"{"compilerOptions":{"types":["vite/client"]}}"#,
    )
    .expect("compiler config should be written");
    std::fs::write(
        root.join("apps/web/src/main.ts"),
        "declare module \"virtual:perish/views\";\nimport source from 'virtual:perish/views'; import { Views } from '@perishlab/design'; mount(Views, { props: { source } });",
    )
    .expect("entry should be written");
    std::fs::write(
        root.join("apps/web/src/views/index.svelte"),
        "export default function Home() { return null; }",
    )
    .expect("view should be written");
    std::fs::write(
        root.join(".forgejo/workflows/guard.yml"),
        "name: guard\njobs:\n  guard:\n    steps:\n      - run: pnpm --filter @specimen/web build\n",
    )
    .expect("guard should be written");
    (scratch, root)
}

#[test]
fn standalone() {
    let (_scratch, root) = fixture("plumb-web-standalone");
    let out = run(&root);
    assert!(!out.contains("[web]"), "{out}");
}

#[test]
fn hosted() {
    let (_scratch, root) = fixture("plumb-web-hosted");
    std::fs::create_dir_all(root.join(".github/workflows")).expect("github should be made");
    std::fs::rename(
        root.join(".forgejo/workflows/guard.yml"),
        root.join(".github/workflows/quality.yml"),
    )
    .expect("guard should move");
    let out = run(&root);
    assert!(!out.contains("[web]"), "{out}");
}

#[test]
fn roles() {
    let (_scratch, root) = fixture("plumb-web-roles");
    std::fs::write(root.join("apps/web/src/views/helper.ts"), "")
        .expect("helper should be written");
    std::fs::write(root.join("apps/web/src/lib/Panel.svelte"), "")
        .expect("component should be written");
    std::fs::write(root.join("apps/web/src/lib/hooks/Hush.ts"), "")
        .expect("hook should be written");
    std::fs::write(root.join("apps/web/src/lib/hooks/hush.tsx"), "")
        .expect("hook should be written");
    std::fs::write(root.join("apps/web/src/lib/components/card.ts"), "")
        .expect("component should be written");
    std::fs::write(root.join("apps/web/src/lib/components/card.scss"), "")
        .expect("style should be written");
    let out = run(&root);
    for line in [
        "web views only hold route svelte files",
        "web lib svelte must live under lib/components",
        "web convention paths must be lowercase",
        "web hooks must be lowercase .ts files",
        "web components only hold lowercase svelte files",
    ] {
        assert!(out.contains(&format!("{line} [web]")), "{out}");
    }
}

#[test]
fn bindings() {
    for (source, finding) in [
        (
            "// design(\nexport default {plugins:[]};",
            "out of true: vite does not activate the design plugin",
        ),
        (
            "import {design} from 'other'; export default {plugins:[design()]};",
            "out of true: vite does not activate the design plugin",
        ),
        (
            "import {design} from '@perishlab/design/vite'; export default () => ({plugins:[design()]});",
            "unknown shape: Vite activation was not qualified:",
        ),
    ] {
        let (_scratch, root) = fixture("plumb-web-binding");
        std::fs::write(root.join("apps/web/vite.config.ts"), source).unwrap();
        let out = run(&root);
        assert!(out.contains(finding), "{out}");
    }
}

#[test]
fn sidecar() {
    let scratch = tempfile::tempdir().expect("fixture");
    let root = scratch.path().join("plumb-dispatch-section");
    for path in [
        "apps/web/src",
        "crates/api/src",
        "deploy",
        "charts/specimen/templates",
        ".forgejo/workflows",
    ] {
        std::fs::create_dir_all(root.join(path)).expect("dir");
    }
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace.package]\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("cargo");
    std::fs::write(
        root.join("crates/api/Cargo.toml"),
        "[package]\nname = \"api\"\nversion = \"0.1.0\"\n",
    )
    .expect("api cargo");
    std::fs::write(
        root.join("crates/api/src/main.rs"),
        "fn main() {\n    let sidecar_stamp = \"\";\n    eprintln!(\"{}\", serde_json::json!({\"role\": \"api\", \"endpoint\": sidecar_stamp}));\n    let _router = Router::new().nest(\"/api\", api);\n}\n",
    )
    .expect("api source");
    std::fs::write(
        root.join("apps/web/package.json"),
        r#"{"name":"@specimen/web","scripts":{"build":"vite build"},"dependencies":{"@perish/design":"0.2.1","svelte":"5"},"devDependencies":{"vite":"7"}}"#,
    )
    .expect("web package");
    std::fs::write(root.join("deploy/api.Dockerfile"), "FROM scratch\n").expect("api image");
    std::fs::write(
        root.join("deploy/web.Dockerfile"),
        "FROM node\nCMD [\"node\", \"dist/.perish/server.mjs\", \"dist\"]\n",
    )
    .expect("web image");
    std::fs::write(
        root.join("charts/specimen/Chart.yaml"),
        "version: 0.1.0\nappVersion: \"0.1.0\"\n",
    )
    .expect("chart");
    std::fs::write(
        root.join("charts/specimen/templates/api.yaml"),
        "kind: Deployment\nmetadata:\n  name: specimen-api\n",
    )
    .expect("api workload");
    std::fs::write(
        root.join("charts/specimen/templates/web.yaml"),
        "kind: Deployment\nmetadata:\n  name: specimen-web\n",
    )
    .expect("web workload");
    std::fs::write(
        root.join("charts/specimen/templates/ingress.yaml"),
        "kind: Ingress\nspec:\n  rules:\n    - http:\n        paths:\n          - path: /api\n            backend:\n              service:\n                name: specimen-api\n          - path: /\n            backend:\n              service:\n                name: specimen-web\n",
    )
    .expect("ingress");
    let valid = "[[sidecar.sidecars]]\nname = \"api\"\nport = 0\nenv = { KEEL_LISTEN_PORT = \"{port}\" }\nhealth_url = \"http://127.0.0.1:{port}/api/health\"\nready = { role = \"api\" }\n\n[sidecar.app]\nname = \"web\"\nport = 0\nhealth_url = \"http://127.0.0.1:{port}\"\ninherits_env = [{ name = \"API_URL\", from = \"api.endpoint\" }]\n";
    std::fs::write(root.join("plumb.toml"), valid).expect("plumb");
    let clean = run(&root);
    assert!(!clean.contains("[dispatch]"), "{clean}");

    std::fs::write(
        root.join("plumb.toml"),
        valid
            .replace("/api/health", "/health")
            .replace("API_URL", "WRONG"),
    )
    .expect("broken section");
    let drifted = run(&root);
    for line in [
        "sidecar api health_url must target {port}/api/health",
        "sidecar web must inherit api.endpoint as API_URL",
    ] {
        assert!(drifted.contains(&format!("{line} [dispatch]")), "{drifted}");
    }

    std::fs::write(root.join("plumb.toml"), valid).expect("plumb");
    std::fs::write(root.join("sidecar.toml"), "[app]\nname = \"web\"\n").expect("legacy");
    let clash = run(&root);
    assert!(clash.contains("blind:"), "{clash}");
    assert!(
        clash.contains("both sidecar.toml and [sidecar] in plumb.toml"),
        "{clash}"
    );
    assert!(clash.contains("[dispatch]"), "{clash}");

    std::fs::remove_file(root.join("sidecar.toml")).expect("remove legacy");
    std::fs::write(root.join("plumb.toml"), "sidecar = 7\n").expect("scalar");
    let bad = run(&root);
    assert!(bad.contains("blind:"), "{bad}");
    assert!(bad.contains("cannot read [sidecar] in plumb.toml"), "{bad}");
}
