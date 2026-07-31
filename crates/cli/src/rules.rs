use std::collections::BTreeSet;
use std::sync::LazyLock;

pub struct Rules {
    pub dirs: BTreeSet<String>,
    pub wrappers: BTreeSet<String>,
    pub lanes: BTreeSet<String>,
    pub retired: Vec<(String, String)>,
    pub blacklist: BTreeSet<String>,
    pub stable: Stable,
}

pub struct Stable {
    pub jsr: Jsr,
    pub cargo: Cargo,
}

pub struct Jsr {
    pub scope: String,
    pub authority: String,
}

pub struct Cargo {
    pub registry: String,
    pub index: String,
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
    let stable = deps
        .get("stable")
        .unwrap_or_else(|| panic!("rules/deps.toml must name stable authorities"));
    let jsr = stable
        .get("jsr")
        .unwrap_or_else(|| panic!("rules/deps.toml must name stable.jsr"));
    let cargo = stable
        .get("cargo")
        .unwrap_or_else(|| panic!("rules/deps.toml must name stable.cargo"));
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
    Rules {
        dirs: set("dirs"),
        wrappers: set("wrappers"),
        lanes: set("lanes"),
        retired,
        blacklist,
        stable: Stable {
            jsr: Jsr {
                scope: required(jsr, "scope"),
                authority: required(jsr, "authority"),
            },
            cargo: Cargo {
                registry: required(cargo, "registry"),
                index: required(cargo, "index"),
            },
        },
    }
});

fn required(value: &toml::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(toml::Value::as_str)
        .unwrap_or_else(|| panic!("rules/deps.toml stable authority must name {key}"))
        .to_string()
}
