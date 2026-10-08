use super::world::World;
use serde_json::json;

#[test]
fn package() {
    let world = World::new();
    world.check(false);
    for (field, value) in [
        ("name", json!("wrong")),
        ("name", json!(null)),
        ("private", json!(false)),
        ("private", json!("true")),
        ("scripts", json!(null)),
        ("scripts", json!({})),
        ("scripts", json!({"build":"  "})),
        ("scripts", json!({"build":true})),
        ("scripts", json!({"build":"node build.mjs","prebuild":""})),
        (
            "scripts",
            json!({"build":"node build.mjs","postbuild":null}),
        ),
    ] {
        let mut held = World::package();
        held[field] = value;
        world.write("package.json", &held.to_string());
        world.check(true);
    }
    for field in ["name", "private", "scripts"] {
        let mut held = World::package();
        held.as_object_mut().unwrap().remove(field);
        world.write("package.json", &held.to_string());
        world.check(true);
    }
    let mut held = World::package();
    held["dependencies"] = json!({"example":"1"});
    held["extra"] = json!([true, 1, 1.5, null, {"quoted":"\\\"}"}]);
    world.write("package.json", &held.to_string());
    world.check(false);
}

#[test]
fn worker() {
    let world = World::new();
    for (field, value) in [
        ("name", json!("Bad")),
        ("name", json!("-bad")),
        ("name", json!("bad-")),
        ("name", json!("a".repeat(64))),
        ("name", json!(null)),
        ("account_id", json!("A".repeat(32))),
        ("account_id", json!("a".repeat(31))),
        ("workers_dev", json!(true)),
        ("workers_dev", json!("false")),
        ("previews", json!({"one":{}})),
        ("previews", json!([])),
        (
            "assets",
            json!({"directory":"dist","not_found_handling":"single-page-application"}),
        ),
        (
            "assets",
            json!({"directory":"dist","not_found_handling":"404-page","extra":true}),
        ),
        ("assets", json!(null)),
        ("main", json!("worker.js")),
        ("routes", json!([])),
        ("bindings", json!({})),
    ] {
        let mut held = World::worker();
        held[field] = value;
        world.write("wrangler.jsonc", &held.to_string());
        world.check(true);
    }
    for field in [
        "name",
        "account_id",
        "compatibility_date",
        "workers_dev",
        "assets",
        "previews",
    ] {
        let mut held = World::worker();
        held.as_object_mut().unwrap().remove(field);
        world.write("wrangler.jsonc", &held.to_string());
        world.check(true);
    }
}

#[test]
fn dates() {
    let world = World::new();
    for (date, refused) in [
        ("2000-02-29", false),
        ("2024-02-29", false),
        ("1900-02-29", true),
        ("2025-02-29", true),
        ("0000-01-01", true),
        ("2026-04-31", true),
        ("2026-00-01", true),
        ("2026-13-01", true),
        ("2026-01-00", true),
        ("2026-1-01", true),
        ("2026-10-08T00:00:00Z", true),
        ("２０２６-01-01", true),
    ] {
        let mut held = World::worker();
        held["compatibility_date"] = json!(date);
        world.write("wrangler.jsonc", &held.to_string());
        world.check(refused);
    }
}

#[test]
fn paths() {
    let world = World::new();
    for (path, refused) in [
        ("./dist", false),
        ("out/site", false),
        ("", true),
        ("/dist", true),
        ("C:/dist", true),
        ("out\\site", true),
        ("../dist", true),
        (".cache", true),
        ("out/.hidden", true),
        ("out/./site", true),
        ("out//site", true),
        ("dist/", true),
        ("././dist", true),
    ] {
        let mut held = World::worker();
        held["assets"]["directory"] = json!(path);
        world.write("wrangler.jsonc", &held.to_string());
        world.check(refused);
    }
}

#[test]
fn documents() {
    let world = World::new();
    let package = World::package().to_string();
    let worker = World::worker().to_string();
    for text in [
        package.replacen("{", "{\"private\":true,", 1),
        package.replacen("{", "{\"priva\\u0074e\":true,", 1),
        package.replace("\"scripts\":{", "\"scripts\":{\"build\":\"other\","),
        format!("// comment\n{package}"),
        format!("{package}{}", " ".repeat(1_048_576)),
        "{".into(),
    ] {
        world.write("package.json", &text);
        world.check(true);
    }
    world.write("package.json", &package);
    world.write("wrangler.jsonc", &format!("  // comment\n{worker}\n// end"));
    world.check(false);
    for text in [
        worker.replacen("{", "{\"name\":\"other\",", 1),
        worker.replace("\"assets\":{", "\"assets\":{\"directory\":\"other\","),
        worker.replace("\"assets\":{", "\"assets\":{/* comment */"),
        worker.replace("\"assets\":{", "\"assets\":{// comment\n"),
        format!("{worker}{}", " ".repeat(1_048_576)),
    ] {
        world.write("wrangler.jsonc", &text);
        world.check(true);
    }
}

#[test]
fn aliases() {
    for path in ["package.json", "other/package.json"] {
        let world = World::new();
        world.alias(path, &World::package().to_string());
        world.check(true);
        world.alias(path, r#"{"name":"different"}"#);
        world.check(false);
        world.alias(path, "null");
        world.check(true);
    }
    for path in ["wrangler.jsonc", "other/wrangler.jsonc"] {
        let world = World::new();
        world.alias(path, &World::worker().to_string());
        world.check(true);
        let mut held = World::worker();
        held["name"] = json!("different");
        held["main"] = json!("runtime.js");
        world.alias(path, &held.to_string());
        world.check(false);
        held["name"] = json!("crest-review");
        held["account_id"] = json!("b".repeat(32));
        world.alias(path, &held.to_string());
        world.check(false);
        world.alias(path, "{");
        world.check(true);
    }
}

#[cfg(unix)]
#[test]
fn output() {
    let world = World::new();
    let output = world.root.path().join("reviews/crest/dist");
    std::fs::create_dir(&output).unwrap();
    world.check(false);
    std::fs::remove_dir(&output).unwrap();
    std::os::unix::fs::symlink("missing", &output).unwrap();
    world.check(true);
    std::fs::remove_file(&output).unwrap();
    std::fs::write(&output, "file").unwrap();
    world.check(true);
    std::fs::remove_file(&output).unwrap();
    let mut held = World::worker();
    held["assets"]["directory"] = json!("dist/nested");
    world.write("wrangler.jsonc", &held.to_string());
    std::os::unix::fs::symlink("missing", &output).unwrap();
    world.check(true);
}
