use super::super as brand;

fn world() -> brand::fixture::Fixture {
    let mut fixture = brand::fixture::Fixture::new();
    fixture.app.mode = brand::Mode::Views;
    fixture.app.setup.options = serde_json::json!({"login":false});
    fixture.value["head"] = serde_json::Value::Null;
    fixture.write();
    std::fs::write(fixture.root.path().join("dist/index.html"), r#"<html><head><link rel="icon" href="data:,"></head><body><div id="root"></div></body></html>"#).unwrap();
    fixture
}

#[test]
fn absent() {
    assert_eq!(world().verify().unwrap(), 2);
    let mut missing = world();
    missing.value.as_object_mut().unwrap().remove("head");
    missing.write();
    assert!(
        missing.verify().is_err(),
        "missing head must differ from explicit null"
    );
    let mut declared = world();
    declared.app.setup.options["brand"] =
        serde_json::json!({"product":"fixture","origin":"https://example.test"});
    assert!(declared.verify().is_err());
}

#[test]
fn icons() {
    for extra in [
        r#"<link rel="icon" href="/crest/a.svg">"#,
        r#"<link rel="ICON" href="/unowned.svg">"#,
        r#"<link rel="apple-touch-icon" href="/crest/a.png">"#,
        r#"<link rel="manifest" href="/app.json">"#,
        r#"<meta property="og:image" content="/crest/a.png">"#,
        r#"<meta name="twitter:image:src" content="/unknown.png">"#,
        r#"<meta property="og:image:width" content="180">"#,
        r#"<meta name="twitter:card" content="summary">"#,
        r#"<meta property="description" name="twitter:image" content="/unknown.png">"#,
    ] {
        let fixture = world();
        let path = fixture.root.path().join("dist/index.html");
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(path, text.replace("</head>", &format!("{extra}</head>"))).unwrap();
        assert!(fixture.verify().is_err(), "{extra}");
    }
}

#[test]
fn bytes() {
    let fixture = world();
    std::fs::write(fixture.root.path().join("dist/crest/a.svg"), "redrawn").unwrap();
    assert!(fixture.verify().is_err());
    let fixture = world();
    std::fs::write(fixture.root.path().join("dist/other.html"), "<html></html>").unwrap();
    assert!(fixture.verify().is_err());
}

#[test]
fn metadata() {
    for extra in [
        r#"<meta property="og:image:secure_url" content="https://other.test/image.png">"#,
        r#"<meta name="twitter:image:src" content="/other.png">"#,
        r#"<meta property="description" name="twitter:image" content="/other.png">"#,
        r#"<link rel="ICON" href="/other.svg">"#,
        r#"<link rel="manifest" href="/app.json">"#,
        r#"<meta property="og:image:width" content="1200">"#,
        r#"<meta property="og:image:height" content="630">"#,
    ] {
        let fixture = brand::fixture::Fixture::new();
        let path = fixture.root.path().join("dist/index.html");
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(path, text.replace("</head>", &format!("{extra}</head>"))).unwrap();
        assert!(fixture.verify().is_err(), "{extra}");
    }
}
