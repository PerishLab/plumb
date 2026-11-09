use super::super as brand;

const CONFIG: &str = r#"import {design} from '@perishlab/design/vite';import {svelte} from '@sveltejs/vite-plugin-svelte';export default {plugins:[design({login:false}),svelte()]};"#;

fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let app = root.path().join("apps/web");
    std::fs::create_dir_all(&app).unwrap();
    std::fs::write(
        app.join("package.json"),
        r#"{"name":"@fixture/web","dependencies":{"svelte":"5"},"devDependencies":{"vite":"8"}}"#,
    )
    .unwrap();
    std::fs::write(app.join("vite.config.ts"), CONFIG).unwrap();
    root
}

#[test]
fn optional() {
    let root = fixture();
    let source = brand::source::Source(root.path());
    assert_eq!(source.apps(brand::Mode::Views).unwrap().len(), 1);
    assert!(
        source.apps(brand::Mode::Html).is_err(),
        "default HTML still requires public head"
    );
}

#[test]
fn compiler() {
    for config in [
        CONFIG.replace("svelte()", "svelte({})"),
        CONFIG.replace("svelte()", "svelte({preprocess:unknown})"),
        CONFIG.replace(",svelte()", ""),
        CONFIG.replace("svelte()", "svelte(),svelte()"),
        CONFIG.replace("import {svelte}", "import type {svelte}"),
    ] {
        let root = fixture();
        std::fs::write(root.path().join("apps/web/vite.config.ts"), &config).unwrap();
        assert!(
            brand::source::Source(root.path())
                .apps(brand::Mode::Views)
                .is_err(),
            "{config}"
        );
    }
}

#[test]
fn redirected() {
    for property in [
        serde_json::json!({"browser":{"@perishlab/design":"./fake.js"}}),
        serde_json::json!({"name":"@perishlab/design","exports":{".":"./fake.js"}}),
        serde_json::json!({"name":"@perishlab/crest","exports":{".":"./fake.js"}}),
    ] {
        let root = fixture();
        let path = root.path().join("apps/web/package.json");
        let mut package: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        for (key, value) in property.as_object().unwrap() {
            package[key] = value.clone();
        }
        std::fs::write(path, serde_json::to_vec(&package).unwrap()).unwrap();
        assert!(
            brand::source::Source(root.path())
                .apps(brand::Mode::Views)
                .is_err()
        );
    }
    for extension in ["js", "ts", "mjs", "mts", "cjs", "cts"] {
        let root = fixture();
        std::fs::write(
            root.path()
                .join(format!("apps/web/svelte.config.{extension}")),
            "export default {};",
        )
        .unwrap();
        assert!(
            brand::source::Source(root.path())
                .apps(brand::Mode::Views)
                .is_err()
        );
    }
}

#[cfg(unix)]
#[test]
fn symlink() {
    let root = fixture();
    let config = root.path().join("apps/web/vite.config.ts");
    let other = root.path().join("external.ts");
    std::fs::rename(&config, &other).unwrap();
    std::os::unix::fs::symlink(&other, &config).unwrap();
    assert!(
        brand::source::Source(root.path())
            .apps(brand::Mode::Views)
            .is_err()
    );
}
