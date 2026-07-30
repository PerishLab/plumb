use plumb::config::Cascade;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize, PartialEq, plumb::config::Cascade)]
#[cascade(section)]
#[serde(default)]
struct Bind {
    host: String,
    port: u16,
}

impl Default for Bind {
    fn default() -> Self {
        Bind {
            host: "127.0.0.1".to_string(),
            port: 3000,
        }
    }
}

#[derive(Debug, PartialEq, plumb::config::Cascade)]
struct Rig {
    name: String,
    count: u16,
    #[cascade(section)]
    listen: Bind,
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
            listen: Bind::default(),
            theme: "plain".to_string(),
            tag: None,
            extras: BTreeMap::new(),
        }
    }
}

fn nothing(_: &str) -> Option<String> {
    None
}

#[test]
fn prefixed() {
    assert_eq!(Rig::prefix(), "PLUMB");
}

#[test]
fn defaults() {
    let held = Rig::default().merge(Rig::lookup("RIG", &nothing).expect("env should read"));
    assert_eq!(held, Rig::default());
}

#[test]
fn filed() {
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
fn veiled() {
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
    .merge(Rig::lookup("RIG", &get).expect("env should read"));
    assert_eq!(held.name, "veiled");
    assert_eq!(held.listen.port, 7);
    assert_eq!(held.tag.as_deref(), Some("held"));
}

#[test]
#[cfg(feature = "skill")]
fn locus() {
    let get = |key: &str| match key {
        "PLUMB_LOCUS_ENABLED" => Some("true".to_string()),
        "PLUMB_LOCUS_REPORT_FILE" => Some("/tmp/audit.jsonl".to_string()),
        "PLUMB_LOCUS_TRACE_FILE" => Some("/tmp/trace.json".to_string()),
        "PLUMB_LOCUS_TRACE_ID" => Some("trace-a".to_string()),
        "PLUMB_LOCUS_TARGET_COLLECTORS" => Some("process:id".to_string()),
        _ => None,
    };
    let held = plumb::rig::Rig::default()
        .merge(plumb::rig::Rig::lookup("PLUMB", &get).expect("locus environment should read"));
    assert!(held.locus.enabled);
    assert_eq!(
        held.locus.report.file,
        std::path::PathBuf::from("/tmp/audit.jsonl")
    );
    assert_eq!(
        held.locus.trace.file,
        std::path::PathBuf::from("/tmp/trace.json")
    );
    assert_eq!(held.locus.trace.id, "trace-a");
    assert_eq!(held.locus.target.collectors, "process:id");
}

#[test]
fn armed() {
    let get = |key: &str| (key == "RIG_THEME").then(|| "enved".to_string());
    let args = RigArgs {
        theme: Some("dark".to_string()),
        tag: Some("tipped".to_string()),
    };
    let held = Rig::default()
        .merge(Rig::lookup("RIG", &get).expect("env should read"))
        .merge(args.partial());
    assert_eq!(held.theme, "dark");
    assert_eq!(held.tag.as_deref(), Some("tipped"));
}

#[test]
fn optional() {
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
fn refused() {
    let get = |key: &str| (key == "RIG_COUNT").then(|| "nope".to_string());
    let err = Rig::lookup("RIG", &get).expect_err("the parse should fail");
    assert!(err.to_string().contains("RIG_COUNT"), "{err}");
}

#[test]
fn blank() {
    let get = |key: &str| (key == "RIG_NAME").then(|| "  ".to_string());
    let held = Rig::default().merge(Rig::lookup("RIG", &get).expect("env should read"));
    assert_eq!(held.name, "base");
}

#[test]
fn blind() {
    let get = |key: &str| (key == "RIG_EXTRAS").then(|| "poked".to_string());
    let held = Rig::default().merge(Rig::lookup("RIG", &get).expect("env should read"));
    assert!(held.extras.is_empty());
}

#[test]
fn onion() {
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
