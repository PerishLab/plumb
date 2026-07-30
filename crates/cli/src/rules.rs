use std::collections::BTreeSet;
use std::sync::LazyLock;

pub struct Rules {
    pub dirs: BTreeSet<String>,
    pub wrappers: BTreeSet<String>,
    pub required: BTreeSet<String>,
    pub hooks: BTreeSet<String>,
    pub lanes: BTreeSet<String>,
    pub retired: Vec<(String, String)>,
    pub blacklist: BTreeSet<String>,
    pub scope: String,
    pub support: Vec<Support>,
}

pub struct Support {
    pub name: String,
    pub requirement: String,
    pub minimum: String,
    pub legacy: String,
    pub line: String,
}

impl Rules {
    pub fn pinned(&self, deno: &str) -> BTreeSet<String> {
        let marker = format!("jsr:{}/", self.scope);
        let mut found = BTreeSet::new();
        for chunk in deno.split(marker.as_str()).skip(1) {
            let name: String = chunk
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
                .collect();
            let package = format!("{}/{name}", self.scope);
            if chunk[name.len()..].starts_with('@')
                && !self.support.iter().any(|support| support.name == package)
            {
                found.insert(package);
            }
        }
        found
    }

    pub fn support(&self, name: &str) -> Option<&Support> {
        self.support.iter().find(|support| support.name == name)
    }
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
    let scope = deps
        .get("scope")
        .and_then(toml::Value::as_str)
        .unwrap_or_default()
        .to_string();
    let blacklist = deps
        .get("blacklist")
        .and_then(toml::Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(toml::Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let support = deps
        .get("support")
        .and_then(toml::Value::as_array)
        .map(|list| {
            list.iter()
                .map(|entry| Support {
                    name: required(entry, "name"),
                    requirement: required(entry, "requirement"),
                    minimum: required(entry, "minimum"),
                    legacy: required(entry, "legacy"),
                    line: required(entry, "line"),
                })
                .collect()
        })
        .unwrap_or_default();
    Rules {
        dirs: set("dirs"),
        wrappers: set("wrappers"),
        required: set("required"),
        hooks: set("hooks"),
        lanes: set("lanes"),
        retired,
        blacklist,
        scope,
        support,
    }
});

fn required(value: &toml::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(toml::Value::as_str)
        .unwrap_or_else(|| panic!("rules/deps.toml support must name {key}"))
        .to_string()
}
