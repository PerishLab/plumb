use crate::command::depot::{Held, held};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

pub struct Rules {
    pub suites: BTreeMap<String, Vec<String>>,
    pub dirs: BTreeSet<String>,
    pub lanes: BTreeSet<String>,
    pub retired: Vec<(String, String)>,
    pub blacklist: BTreeSet<String>,
    pub stable: Stable,
    pub release: Release,
}

pub struct Release {
    pub ceiling: usize,
    pub permitted: BTreeMap<String, usize>,
    pub exercised: BTreeMap<String, usize>,
    pub forge: String,
}

pub struct Stable {
    pub cargo: Cargo,
}

pub struct Cargo {
    pub registry: String,
    pub index: String,
}

pub const SETS: [(&str, &str); 5] = [
    ("deps", plumb::seat::resource!("rules/deps.toml")),
    ("release", plumb::seat::resource!("rules/release.toml")),
    ("seat", plumb::seat::resource!("rules/seat.toml")),
    ("structure", plumb::seat::resource!("rules/structure.toml")),
    ("workflow", plumb::seat::resource!("rules/workflow.toml")),
];

pub static FACTORY: LazyLock<toml::Table> = LazyLock::new(|| {
    shaped(plumb::seat::resource!("rules/policy.toml"))
        .expect("the compiled rules/policy.toml must hold its complete shape")
});

pub static POLICY: LazyLock<toml::Table> = LazyLock::new(|| {
    let factory = plumb::seat::resource!("rules/policy.toml");
    let text = carried(&held(), "rules/policy.toml", factory);
    shaped(&text).unwrap_or_else(|| (*FACTORY).clone())
});

fn shaped(text: &str) -> Option<toml::Table> {
    let table: toml::Table = text.parse().ok()?;
    for name in ["shape", "web"] {
        if table.get(name)?.as_array()?.is_empty() {
            return None;
        }
    }
    Some(table)
}

pub fn limits() -> BTreeMap<String, i64> {
    measured(&POLICY)
}

pub fn factory() -> BTreeMap<String, i64> {
    measured(&FACTORY)
}

fn measured(policy: &toml::Table) -> BTreeMap<String, i64> {
    policy
        .get("limit")
        .and_then(toml::Value::as_table)
        .map(|table| {
            table
                .iter()
                .filter_map(|(name, value)| Some((name.clone(), value.as_integer()?)))
                .collect()
        })
        .unwrap_or_else(|| panic!("rules/policy.toml must hold a limit table"))
}

pub fn setting(section: &str, key: &str) -> bool {
    POLICY
        .get(section)
        .and_then(|table| table.get(key))
        .and_then(toml::Value::as_bool)
        .unwrap_or_else(|| panic!("rules/policy.toml must hold {section}.{key}"))
}

pub fn read(name: &str) -> Result<toml::Table, String> {
    let factory = SETS
        .iter()
        .find(|(held, _)| *held == name)
        .map(|(_, factory)| *factory)
        .ok_or_else(|| format!("rule://{name} names no released set"))?;
    held()
        .read(&format!("rules/{name}.toml"), factory)?
        .parse()
        .map_err(|error| format!("rules/{name}.toml does not parse: {error}"))
}

pub static RULES: LazyLock<Rules> = LazyLock::new(|| {
    let seat = held();
    let doc: toml::Table = carried(
        &seat,
        "rules/structure.toml",
        plumb::seat::resource!("rules/structure.toml"),
    )
    .parse()
    .expect("rules/structure.toml must parse");
    let workflow: toml::Table = carried(
        &seat,
        "rules/workflow.toml",
        plumb::seat::resource!("rules/workflow.toml"),
    )
    .parse()
    .expect("rules/workflow.toml must parse");
    let suites = workflow
        .get("suite")
        .and_then(toml::Value::as_table)
        .map(|table| {
            table
                .iter()
                .map(|(name, value)| {
                    let held = value
                        .as_array()
                        .expect("rules/workflow.toml suite must hold paths")
                        .iter()
                        .filter_map(toml::Value::as_str)
                        .map(str::to_string)
                        .collect();
                    (name.clone(), held)
                })
                .collect()
        })
        .unwrap_or_default();
    let deps: toml::Table = carried(
        &seat,
        "rules/deps.toml",
        plumb::seat::resource!("rules/deps.toml"),
    )
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
    let release: toml::Table = carried(
        &seat,
        "rules/release.toml",
        plumb::seat::resource!("rules/release.toml"),
    )
    .parse()
    .expect("rules/release.toml must parse");
    let forge = release
        .get("forge")
        .and_then(|table| table.get("image"))
        .and_then(toml::Value::as_str)
        .unwrap_or_else(|| panic!("rules/release.toml must name forge.image"))
        .to_string();
    Rules {
        suites,
        dirs: members(&doc, "dir"),
        lanes: members(&doc, "lane"),
        retired,
        blacklist,
        stable: Stable {
            cargo: Cargo {
                registry: required(cargo, "registry"),
                index: required(cargo, "index"),
            },
        },
        release: Release {
            ceiling: ceiling(&release),
            permitted: counted(&release, "permitted"),
            exercised: counted(&release, "exercised"),
            forge,
        },
    }
});

fn carried(seat: &Held, path: &str, factory: &'static str) -> String {
    seat.read(path, factory)
        .unwrap_or_else(|_| factory.to_string())
}

fn members(doc: &toml::Table, key: &str) -> BTreeSet<String> {
    doc.get(key)
        .and_then(|value| value.get("member"))
        .and_then(toml::Value::as_array)
        .map(|list| list.iter().filter_map(permitted).collect())
        .unwrap_or_default()
}

fn permitted(value: &toml::Value) -> Option<String> {
    if value.get("kind").and_then(toml::Value::as_str) == Some("retired") {
        return None;
    }
    value
        .get("name")
        .and_then(toml::Value::as_str)
        .map(str::to_string)
}

fn ceiling(doc: &toml::Table) -> usize {
    doc.get("ceiling")
        .and_then(toml::Value::as_integer)
        .and_then(|held| usize::try_from(held).ok())
        .unwrap_or_else(|| panic!("rules/release.toml must name ceiling"))
}

fn counted(doc: &toml::Table, key: &str) -> BTreeMap<String, usize> {
    doc.get(key)
        .and_then(toml::Value::as_table)
        .map(|table| {
            table
                .iter()
                .filter_map(|(name, value)| {
                    let held = value.as_integer()?;
                    usize::try_from(held).ok().map(|held| (name.clone(), held))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn required(value: &toml::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(toml::Value::as_str)
        .unwrap_or_else(|| panic!("rules/deps.toml stable authority must name {key}"))
        .to_string()
}
