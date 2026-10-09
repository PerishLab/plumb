use super::preview::{PREVIEW, preview, previewed};
use super::world::World;
use super::write;
use serde_json::json;

#[test]
fn bindings() {
    let root = preview();
    let binding = PREVIEW
        .split_once("[lane.app.crest.binding.preview]")
        .unwrap()
        .1;
    for key in ["preview", "preview.a", "preview.crest-a"] {
        let text = PREVIEW.replace("binding.preview", &format!("binding.'{key}'"));
        write(root.path(), "plumb.toml", &text);
        assert!(previewed(root.path()).is_empty(), "{text}");
    }
    let keyed = format!("[lane.app.crest.binding.'preview.a']{binding}").replace(
        "binding.preview.authorities",
        "binding.'preview.a'.authorities",
    );
    write(root.path(), "plumb.toml", &format!("{PREVIEW}{keyed}"));
    assert!(previewed(root.path()).is_empty());
    for key in [
        "Preview",
        "preview.",
        "preview.a.b",
        "preview-a",
        "lane.a",
        "v1.2.3-lane.a",
    ] {
        let text = PREVIEW.replace("binding.preview", &format!("binding.'{key}'"));
        write(root.path(), "plumb.toml", &text);
        assert!(!previewed(root.path()).is_empty(), "{text}");
    }
    write(
        root.path(),
        "plumb.toml",
        &PREVIEW.replace("binding.preview", "binding.preview.a"),
    );
    assert!(!previewed(root.path()).is_empty());
}

#[test]
fn declarations() {
    let root = preview();
    for (from, to) in [
        ("PerishLab/crest", "../crest"),
        ("PerishLab/crest", "owner/repo/extra"),
        ("lane.app.crest", "lane.app.'crest.review'"),
        ("'static'", "'static.key'"),
        ("'static'", "'binary'"),
        ("['inspect', 'deploy']", "[]"),
        ("['inspect', 'deploy']", "['inspect']"),
        ("['inspect', 'deploy']", "['deploy']"),
        ("['inspect', 'deploy']", "['inspect', 'deploy', 'install']"),
        ("['inspect', 'deploy']", "['inspect', 'deploy', 'publish']"),
        (
            "['inspect', 'deploy']",
            "['inspect', 'deploy', 'uninstall']",
        ),
        ("['inspect', 'deploy']", "['inspect', 'deploy', 'repair']"),
        ("['inspect', 'deploy']", "['inspect', 'deploy', 'deploy']"),
        ("'ensign'", "'ensign.key'"),
        ("'wharf'", "'wharf.key'"),
        (
            "account = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'",
            "account = 'bad'",
        ),
        ("resource = 'crest-review'", "resource = 'Crest'"),
        ("resource = 'crest-review'", "resource = '-crest'"),
    ] {
        let text = PREVIEW.replace(from, to);
        write(root.path(), "plumb.toml", &text);
        assert!(!previewed(root.path()).is_empty(), "{text}");
    }
    for section in [
        "[lane]\nrepository = 'PerishLab/crest'\n",
        "[lane]\nrepository = 'PerishLab/crest'\napp = {}\n",
    ] {
        write(root.path(), "plumb.toml", section);
        assert!(!previewed(root.path()).is_empty());
    }
    for start in [
        "adaptor =",
        "capabilities =",
        "authorization =",
        "publication =",
        "repository =",
    ] {
        let text = PREVIEW
            .lines()
            .filter(|line| !line.starts_with(start))
            .collect::<Vec<_>>()
            .join("\n");
        write(root.path(), "plumb.toml", &text);
        assert!(!previewed(root.path()).is_empty());
    }
    for table in [
        "lane",
        "lane.app.crest",
        "lane.app.crest.mapping",
        "lane.app.crest.binding.preview",
        "lane.app.crest.binding.preview.authorities",
    ] {
        let text = PREVIEW.replace(
            &format!("[{table}]"),
            &format!("[{table}]\ntoken = 'secret'"),
        );
        write(root.path(), "plumb.toml", &text);
        assert!(!previewed(root.path()).is_empty());
    }
}

#[test]
fn separation() {
    let world = World::new();
    for text in [
        PREVIEW.replace("lane.app.crest", "lane.app.design"),
        PREVIEW.replace(
            "['inspect', 'deploy']",
            "['dispose', 'deploy', 'inspect', 'build']",
        ),
        PREVIEW.replace("'ensign'", "'wharf'"),
    ] {
        write(world.root.path(), "plumb.toml", &text);
        assert!(previewed(world.root.path()).is_empty());
        world.check(false);
    }
    assert!(!PREVIEW.contains("stamp"));
    assert!(!PREVIEW.contains("url"));
}

#[test]
fn mapping() {
    let world = World::new();
    for (field, value) in [
        ("name", json!("different")),
        ("account_id", json!("b".repeat(32))),
    ] {
        let mut held = World::worker();
        held[field] = value;
        world.write("wrangler.jsonc", &held.to_string());
        assert!(
            world
                .judge()
                .contains("differs from the declared static resource mapping")
        );
    }
    let mut held = World::worker();
    held["name"] = json!("different");
    world.write("wrangler.jsonc", &held.to_string());
    write(
        world.root.path(),
        "plumb.toml",
        &PREVIEW.replace("resource = 'crest-review'", "resource = 'different'"),
    );
    assert!(previewed(world.root.path()).is_empty());
    world.check(false);
}
