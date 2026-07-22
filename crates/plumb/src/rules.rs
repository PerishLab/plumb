use std::collections::BTreeSet;
use std::sync::LazyLock;

pub struct Rules {
    pub dirs: BTreeSet<String>,
    pub wrappers: BTreeSet<String>,
    pub required: BTreeSet<String>,
    pub lanes: BTreeSet<String>,
    pub retired: Vec<(String, String)>,
}

pub static RULES: LazyLock<Rules> = LazyLock::new(|| {
    let doc: toml::Table = include_str!("../rules/structure.toml")
        .parse()
        .expect("rules/structure.toml must parse");
    let set = |key: &str| -> BTreeSet<String> {
        doc.get(key)
            .and_then(toml::Value::as_array)
            .map(|list| {
                list.iter()
                    .filter_map(toml::Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    };
    let deps: toml::Table = include_str!("../rules/deps.toml")
        .parse()
        .expect("rules/deps.toml must parse");
    let retired = deps
        .get("retired")
        .and_then(toml::Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|entry| {
                    let name = entry.get("name").and_then(toml::Value::as_str)?;
                    let held = entry.get("use").and_then(toml::Value::as_str)?;
                    Some((name.to_string(), held.to_string()))
                })
                .collect()
        })
        .unwrap_or_default();
    Rules {
        dirs: set("dirs"),
        wrappers: set("wrappers"),
        required: set("required"),
        lanes: set("lanes"),
        retired,
    }
});
