use plumb::config::{Cascade, Env, Kind, Listen};
use std::collections::BTreeMap;

#[derive(Debug, PartialEq, plumb::config::Cascade)]
struct Rig {
    name: String,
    count: u16,
    #[cascade(section)]
    listen: Listen,
    #[cascade(arg)]
    theme: String,
    #[cascade(arg)]
    tag: Option<String>,
    extras: BTreeMap<String, String>,
}

impl Default for Rig {
    fn default() -> Self {
        Rig {
            name: "base".to_string(),
            count: 4,
            listen: Listen::default(),
            theme: "plain".to_string(),
            tag: None,
            extras: BTreeMap::new(),
        }
    }
}

fn nothing(_key: &str) -> Option<String> {
    None
}

#[test]
fn prefix_is_the_package_name() {
    assert_eq!(Rig::prefix(), "PLUMB");
}

#[test]
fn defaults_hold_when_every_layer_is_silent() {
    let held = Rig::default().merge(Rig::env_with("RIG", &nothing).expect("env should read"));
    assert_eq!(held, Rig::default());
}

#[test]
fn the_file_covers_the_defaults() {
    let dir = std::env::temp_dir().join("plumb-cascade-file");
    std::fs::create_dir_all(&dir).expect("fixture should be made");
    let path = dir.join("rig.toml");
    std::fs::write(
        &path,
        "name = \"filed\"\n[listen]\nport = 9\n[extras]\nkey = \"value\"\n",
    )
    .expect("file should be written");
    let over = plumb::config::load::<RigPartial>(&path).expect("file should parse");
    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
    let held = Rig::default().merge(over);
    assert_eq!(held.name, "filed");
    assert_eq!(held.listen.port, 9);
    assert_eq!(held.listen.host, "127.0.0.1");
    assert_eq!(held.count, 4);
    assert_eq!(held.extras.get("key").map(String::as_str), Some("value"));
}

#[test]
fn env_covers_the_file() {
    let get = |key: &str| match key {
        "RIG_NAME" => Some("veiled".to_string()),
        "RIG_LISTEN_PORT" => Some("7".to_string()),
        "RIG_TAG" => Some("held".to_string()),
        _ => None,
    };
    let held = Rig {
        name: "filed".to_string(),
        ..Rig::default()
    }
    .merge(Rig::env_with("RIG", &get).expect("env should read"));
    assert_eq!(held.name, "veiled");
    assert_eq!(held.listen.port, 7);
    assert_eq!(held.tag.as_deref(), Some("held"));
}

#[test]
fn args_cover_env() {
    let get = |key: &str| (key == "RIG_THEME").then(|| "enved".to_string());
    let args = RigArgs {
        theme: Some("dark".to_string()),
        tag: Some("tipped".to_string()),
    };
    let held = Rig::default()
        .merge(Rig::env_with("RIG", &get).expect("env should read"))
        .merge(args.partial());
    assert_eq!(held.theme, "dark");
    assert_eq!(held.tag.as_deref(), Some("tipped"));
}

#[test]
fn args_are_optional_at_the_parser() {
    #[derive(clap::Parser)]
    struct Cli {
        #[command(flatten)]
        over: RigArgs,
    }
    let bare = <Cli as clap::Parser>::try_parse_from(["rig"]).expect("bare parse should hold");
    assert_eq!(bare.over.theme, None);
    assert_eq!(bare.over.tag, None);
    let full = <Cli as clap::Parser>::try_parse_from(["rig", "--theme", "dark", "--tag", "t"])
        .expect("full parse should hold");
    assert_eq!(full.over.theme.as_deref(), Some("dark"));
    assert_eq!(full.over.tag.as_deref(), Some("t"));
}

#[test]
fn a_bad_env_value_is_an_error_not_a_default() {
    let get = |key: &str| (key == "RIG_COUNT").then(|| "nope".to_string());
    let err = Rig::env_with("RIG", &get).expect_err("the parse should fail");
    assert!(err.to_string().contains("RIG_COUNT"), "{err}");
}

#[test]
fn an_empty_env_value_is_absent() {
    let get = |key: &str| (key == "RIG_NAME").then(|| "  ".to_string());
    let held = Rig::default().merge(Rig::env_with("RIG", &get).expect("env should read"));
    assert_eq!(held.name, "base");
}

#[test]
fn a_map_is_blind_to_env() {
    let get = |key: &str| (key == "RIG_EXTRAS").then(|| "poked".to_string());
    let held = Rig::default().merge(Rig::env_with("RIG", &get).expect("env should read"));
    assert!(held.extras.is_empty());
}

#[test]
fn kind_reads_from_env() {
    assert_eq!(Kind::read("file"), Ok(Kind::File));
    assert_eq!(Kind::read("memory"), Ok(Kind::Memory));
    assert!(Kind::read("shelf").is_err());
}

#[test]
fn resolve_runs_the_whole_onion() {
    let dir = std::env::temp_dir().join("plumb-cascade-resolve");
    std::fs::create_dir_all(&dir).expect("fixture should be made");
    let path = dir.join("rig.toml");
    std::fs::write(&path, "name = \"filed\"\ncount = 2\n").expect("file should be written");
    let held = Rig::resolve(Some(&path)).expect("the onion should resolve");
    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
    assert_eq!(held.name, "filed");
    assert_eq!(held.count, 2);
    assert_eq!(held.theme, "plain");
}
