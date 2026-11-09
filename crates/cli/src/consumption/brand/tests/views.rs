use super::super as brand;
use std::path::Path;

const ENTRY: &str = r#"
import source from 'virtual:perish/views';
import { Views } from '@perishlab/design';
import { mount } from 'svelte';
// The whitespace and quote style differ from the recognizer's representation.
const target = document.getElementById('root');
if (target !== null) { mount(Views, { target, props: { source } }); }
"#;

const VIEW: &str = r#"<script lang="ts">import {Hero,Shell,Frame} from '@perishlab/design';</script><Shell><Frame><Hero title="Fixture" line="Line" mark="fixture"/></Frame></Shell>"#;
const HTML: &str = r#"<!doctype html><html><head><meta charset="UTF-8"><link rel="icon" href="data:,"></head><body><div id="root"></div><script type="module" src="/src/main.ts"></script></body></html>"#;

struct World {
    root: tempfile::TempDir,
    app: brand::source::App,
}

impl World {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("src/views")).unwrap();
        std::fs::write(root.path().join("src/main.ts"), ENTRY).unwrap();
        std::fs::write(root.path().join("src/views/index.svelte"), VIEW).unwrap();
        std::fs::write(root.path().join("index.html"), HTML).unwrap();
        Self {
            root,
            app: brand::source::App {
                path: "".into(),
                config: "vite.config.ts".into(),
                mode: brand::Mode::Views,
                setup: crate::shape::web::Setup {
                    options: serde_json::json!({"login":false}),
                    base: "/".into(),
                    output: "dist".into(),
                    problem: None,
                },
            },
        }
    }

    fn verify(&self) -> Result<(), String> {
        let mut sources = Vec::new();
        for file in brand::source::Source(&self.root.path().join("src")).files()? {
            let text = std::fs::read_to_string(&file).unwrap();
            sources.push((file, brand::svelte::read(&text)?));
        }
        brand::source::Source(self.root.path()).views(&self.app, &sources)
    }

    fn write(&self, path: &str, text: &str) {
        std::fs::write(self.root.path().join(path), text).unwrap();
    }
}

#[test]
fn standard() {
    World::new().verify().unwrap();
    let mut world = World::new();
    world.app.setup.options["brand"] =
        serde_json::json!({"product":"fixture","origin":"https://example.test"});
    world.verify().unwrap();
    world.app.setup.options["brand"]["product"] = "other".into();
    assert!(world.verify().is_err());
    world.app.setup.base = "/prefix/".into();
    assert!(world.verify().is_err());
}

#[test]
fn imports() {
    for entry in [
        "// import source, Views and mount; nothing actually mounts",
        &ENTRY.replace("@perishlab/design", "business"),
        &ENTRY.replace("import { mount }", "import type { mount }"),
        &ENTRY.replace("mount(Views,", "mount(Other,"),
        &ENTRY.replace("props: { source }", "props: { source: other }"),
        &ENTRY.replace("mount(Views,", "if (false) mount(Views,"),
        &format!("{ENTRY}\nlocation.replace('/elsewhere');"),
    ] {
        let world = World::new();
        world.write("src/main.ts", entry);
        assert!(world.verify().is_err(), "{entry}");
    }
}

#[test]
fn unreachable() {
    for view in [
        &VIEW
            .replace("<Hero title=", "{#if false}<Hero title=")
            .replace("/></Frame>", "/>{/if}</Frame>"),
        &VIEW.replace("from '@perishlab/design'", "from 'business'"),
        &VIEW.replace("<Frame>", "<Frame width=\"hidden\">"),
        &VIEW
            .replace("<Frame>", "<Wrapper>")
            .replace("</Frame>", "</Wrapper>"),
    ] {
        let world = World::new();
        world.write("src/views/index.svelte", view);
        assert!(world.verify().is_err(), "{view}");
    }
    let world = World::new();
    world.write("src/views/other.svelte", VIEW);
    assert!(world.verify().is_err());
    let world = World::new();
    world.write("src/unused.svelte", VIEW);
    assert!(world.verify().is_err());
}

#[test]
fn document() {
    for html in [
        &HTML.replace("id=\"root\"", "id=\"other\""),
        &HTML.replace(
            "<div id=\"root\"></div>",
            "<div id=\"root\"></div><div id=\"root\"></div>",
        ),
        &HTML.replace("type=\"module\"", "type=\"importmap\""),
        &HTML.replace("src=\"/src/main.ts\"", "src=\"https://other.test/main.ts\""),
        &HTML.replace("<body>", "<body onload=\"redirect()\">"),
        &HTML.replace("<head>", "<head><base href=\"https://other.test\">"),
        &HTML.replace("</body>", "<script>mount(Views)</script></body>"),
        "<!-- <div id='root'></div><script type='module' src='/src/main.ts'></script> -->",
    ] {
        let world = World::new();
        world.write("index.html", html);
        assert!(world.verify().is_err(), "{html}");
    }
}

#[test]
fn output() {
    let world = World::new();
    world.verify().unwrap();
    let source = brand::source::Source(world.root.path());
    assert!(source.built(&world.app).is_err());
    std::fs::create_dir_all(world.root.path().join("dist/assets")).unwrap();
    world.write(
        "dist/index.html",
        &HTML.replace("/src/main.ts", "/assets/main.js"),
    );
    assert!(source.built(&world.app).is_err());
    world.write("dist/assets/main.js", "export {};");
    let entry = source.built(&world.app).unwrap();
    assert_eq!(entry.source, "/src/main.ts");
    assert_eq!(entry.output, "/assets/main.js");
    assert!(entry.digest.starts_with("sha256:"));
    world.write("dist/assets/main.js", "");
    assert!(source.built(&world.app).is_err());
    assert!(Path::new(&entry.view).ends_with("index.svelte"));
}
