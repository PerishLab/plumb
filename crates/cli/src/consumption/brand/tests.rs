use super::svelte;
#[test]
fn imported() {
    let marks = svelte::read(r#"<script lang="ts">import {Hero as Brand} from '@perishlab/design';</script><Brand mark="plumb" title="Plumb" line="Line" />"#).unwrap();
    assert_eq!(marks.len(), 1);
    assert_eq!(marks[0].product, "plumb");
}
#[test]
fn absent() {
    for text in [
        r#"<!-- <Hero mark="plumb"/> --><svg><path d="M0 0"/></svg>"#,
        r#"<script>import {Hero} from 'other';</script><Hero mark="plumb"/>"#,
    ] {
        assert!(svelte::read(text).unwrap().is_empty());
    }
}
#[test]
fn dynamic() {
    for text in [
        r#"<script>import {Hero} from '@perishlab/design';</script><Hero mark={unknown}/>"#,
        r#"<script>import {Hero} from '@perishlab/design';</script><Hero mark="plumb" {...props}/>"#,
    ] {
        assert!(svelte::read(text).is_err(), "{text}");
    }
}

#[test]
fn hidden() {
    for text in [
        r#"<script module>import {Hero} from '@perishlab/design';</script><Hero mark="plumb"/>"#,
        r#"<script context="module">import {Hero} from '@perishlab/design';</script><Hero mark="plumb"/>"#,
        r#"<script>import {Hero} from '@perishlab/design';</script>{#each things as Hero}<Hero mark="plumb"/>{/each}"#,
    ] {
        assert!(svelte::read(text).is_err(), "{text}");
    }
    for text in [
        r#"<script>import /*comment*/ type {Hero} from '@perishlab/design';</script><Hero mark="plumb"/>"#,
        r#"<script>import {type /*comment*/ Hero} from '@perishlab/design';</script><Hero mark="plumb"/>"#,
    ] {
        assert!(svelte::read(text).unwrap().is_empty(), "{text}");
    }
}

fn image(medium: &str) -> super::receipt::Asset {
    super::receipt::Asset {
        id: format!("fixture/{medium}"),
        product: "fixture".into(),
        kind: "shared".into(),
        form: "sign".into(),
        step: "full".into(),
        medium: medium.into(),
        surface: "light".into(),
        width: 100,
        height: 100,
        file: format!("media/fixture.{medium}"),
        digest: "sha256:fixture".into(),
    }
}

#[test]
fn head() {
    use std::collections::BTreeMap;
    let outputs = BTreeMap::from([
        ("/crest/a.svg".into(), vec![image("svg")]),
        ("/crest/a.png".into(), vec![image("png")]),
    ]);
    let text = r#"<html><head><link rel="icon" href="/crest/a.svg"><link rel="apple-touch-icon" href="/crest/a.png"><meta property="og:image" content="https://example.test/crest/a.png"><meta name="twitter:card" content="summary"><meta name="twitter:image" content="https://example.test/crest/a.png"></head><body><svg><path d="M0 0"/></svg></body></html>"#;
    super::head::read(
        text,
        &super::receipt::Selection {
            product: "fixture".into(),
            origin: "https://example.test".into(),
        },
        &outputs,
    )
    .unwrap();
    for bad in [
        text.replace("/crest/a.svg", "/favicon.svg"),
        text.replace(
            "https://example.test/crest/a.png",
            "https://unrelated.test/image.png",
        ),
        text.replace("summary", "summary_large_image"),
        text.replace(
            "apple-touch-icon\" href=\"/crest/a.png",
            "apple-touch-icon\" href=\"/crest/a.svg",
        ),
        text.replace("<head>", "<head><!--")
            .replace("</head>", "--></head>"),
        text.replace("<head>", "<body>")
            .replace("</head>", "</body>"),
    ] {
        assert!(
            super::head::read(
                &bad,
                &super::receipt::Selection {
                    product: "fixture".into(),
                    origin: "https://example.test".into()
                },
                &outputs
            )
            .is_err(),
            "{bad}"
        );
    }
}

#[test]
fn configuration() {
    use crate::shape::web::Activation;
    for source in [
        r#"import {design} from '@perishlab/design/vite';export default {plugins:[design()],resolve:{alias:{'@perishlab/design':'./fake'}}};"#,
        r#"import {design} from '@perishlab/design/vite';import {fake} from './fake';export default {plugins:[design(),fake()]};"#,
        r#"import {design} from '@perishlab/design/vite';export default {plugins:[design()],build:{rollupOptions:{input:'./other.html'}}};"#,
    ] {
        let Activation::Active(setup) = Activation::read(source) else {
            panic!("real imported call must remain an activation observation")
        };
        assert!(setup.problem.is_some(), "{source}");
    }
}

#[test]
fn receipt() {
    let fixture = super::fixture::Fixture::new();
    assert_eq!(fixture.verify().unwrap(), 2);
    for field in ["schema", "package", "model", "manifest", "head", "assets"] {
        let mut bad = super::fixture::Fixture::new();
        bad.value[field] = serde_json::Value::Null;
        bad.write();
        assert!(bad.verify().is_err(), "{field}");
    }
    for field in [
        "id", "product", "kind", "file", "digest", "width", "height", "medium", "surface", "step",
        "form", "output",
    ] {
        let mut bad = super::fixture::Fixture::new();
        bad.value["assets"][0][field] = "unqualified".into();
        bad.write();
        assert!(bad.verify().is_err(), "{field}");
    }
    for field in ["unknown", "package"] {
        let mut bad = super::fixture::Fixture::new();
        bad.value["assets"][0][field] = "unexpected".into();
        bad.write();
        assert!(bad.verify().is_err(), "extra output field {field}");
    }
}

#[test]
fn corrupted() {
    let fixture = super::fixture::Fixture::new();
    std::fs::write(
        fixture.root.path().join("dist/crest/a.svg"),
        "altered pixels",
    )
    .unwrap();
    assert!(fixture.verify().is_err());
    let mut fixture = super::fixture::Fixture::new();
    let item = fixture.value["assets"][0].clone();
    fixture.value["assets"].as_array_mut().unwrap().push(item);
    fixture.write();
    assert!(fixture.verify().is_err());
    let fixture = super::fixture::Fixture::new();
    let path = fixture.root.path().join("dist/index.html");
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(path, text.replace("/crest/a.svg", "/public/favicon.svg")).unwrap();
    assert!(fixture.verify().is_err());
    let fixture = super::fixture::Fixture::new();
    std::fs::remove_file(fixture.root.path().join("dist/crest/a.png")).unwrap();
    assert!(fixture.verify().is_err());
}

#[test]
fn copies() {
    let fixture = super::fixture::Fixture::new();
    let target = fixture.root.path().join("public");
    std::fs::create_dir(&target).unwrap();
    std::fs::copy(
        fixture.root.path().join("dist/crest/a.svg"),
        target.join("favicon.svg"),
    )
    .unwrap();
    for args in [vec!["init", "-q"], vec!["add", "public/favicon.svg"]] {
        let status = plumb::config::detached("git")
            .args(args)
            .current_dir(fixture.root.path())
            .status()
            .unwrap();
        assert!(status.success());
    }
    let mut prepared = super::Prepared {
        declaration: super::Declaration {
            copy: vec![super::Copy {
                asset: "fixture/svg".into(),
                output: "public/favicon.svg".into(),
            }],
        },
        apps: Vec::new(),
        materials: fixture.materials,
        inputs: "fixture".into(),
    };
    assert_eq!(prepared.verify(fixture.root.path()).unwrap().copies, 1);
    prepared.declaration.copy[0].asset = "unregistered".into();
    assert!(prepared.verify(fixture.root.path()).is_err());
    prepared.declaration.copy[0].asset = "fixture/svg".into();
    std::fs::write(target.join("favicon.svg"), "hand drawn").unwrap();
    assert!(prepared.verify(fixture.root.path()).is_err());
}

#[test]
fn discovery() {
    let root = tempfile::tempdir().unwrap();
    for name in ["web", "new-product"] {
        let directory = root.path().join("apps").join(name);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            directory.join("package.json"),
            r#"{"dependencies":{"svelte":"5"},"devDependencies":{"vite":"8"}}"#,
        )
        .unwrap();
        std::fs::write(directory.join("vite.config.ts"),r#"import {design} from '@perishlab/design/vite';export default {plugins:[design({brand:{product:'fixture',origin:'https://example.test'}})]};"#).unwrap();
    }
    let apps = super::source::Source(root.path()).apps().unwrap();
    assert_eq!(apps.len(), 2);
    assert!(apps.iter().any(|app| app.path.ends_with("new-product")));
    std::fs::write(
        root.path().join("apps/new-product/vite.config.ts"),
        "// design(\nexport default {plugins:[]};",
    )
    .unwrap();
    assert!(super::source::Source(root.path()).apps().is_err());
}

#[test]
fn inputs() {
    let root = tempfile::tempdir().unwrap();
    let base = root.path();
    for dir in ["src", "crates", "public"] {
        std::fs::create_dir(base.join(dir)).unwrap();
    }
    std::fs::write(base.join("src/entry.svelte"), "<Hero />").unwrap();
    std::fs::write(base.join("crates/lib.rs"), "fn original(){}").unwrap();
    std::fs::write(base.join("public/image.svg"), "original image").unwrap();
    for args in [vec!["init", "-q"], vec!["add", "."]] {
        assert!(
            plumb::config::detached("git")
                .args(args)
                .current_dir(base)
                .status()
                .unwrap()
                .success()
        );
    }
    let apps = vec![super::source::App {
        path: "".into(),
        setup: crate::shape::web::Setup {
            options: serde_json::json!({}),
            base: "/".into(),
            output: "dist".into(),
            problem: None,
        },
    }];
    let source = super::source::Source(base);
    let before = source.inputs(&apps).unwrap();
    std::fs::write(base.join("src/entry.svelte"), "<Hero mark=\"fixture\" />").unwrap();
    let changed = source.inputs(&apps).unwrap();
    assert_ne!(before, changed);
    std::fs::write(base.join("crates/lib.rs"), "fn unrelated_rust_change(){}").unwrap();
    assert_eq!(changed, source.inputs(&apps).unwrap());
    std::fs::write(base.join("public/image.svg"), "changed image").unwrap();
    assert_ne!(changed, source.inputs(&apps).unwrap());
    std::fs::write(base.join("public/untracked.svg"), "unknown").unwrap();
    assert!(source.inputs(&apps).is_err());
}

#[test]
fn bound() {
    let fixture = super::fixture::Fixture::new();
    let prepared = super::Prepared {
        declaration: super::Declaration { copy: Vec::new() },
        apps: Vec::new(),
        materials: fixture.materials,
        inputs: "fixture source".into(),
    };
    let input = prepared.bind(&"a".repeat(64)).unwrap();
    assert_eq!(input.len(), 64, "Guard action input hash ABI");
    assert!(input.bytes().all(|byte| byte.is_ascii_hexdigit()));
}
