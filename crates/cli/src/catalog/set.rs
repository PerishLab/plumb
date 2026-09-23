use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

pub struct Rules {
    pub dirs: BTreeSet<String>,
    pub lanes: BTreeSet<String>,
    pub retired: Vec<(String, String)>,
    pub blacklist: BTreeSet<String>,
    pub release: Release,
}

pub struct Cargo {
    pub registry: &'static str,
    pub index: &'static str,
}

pub const CARGO: Cargo = Cargo {
    registry: "perish",
    index: "sparse+https://git.perish.top/api/packages/PerishLab/cargo/",
};

pub struct Release {
    pub ceiling: usize,
    pub permitted: BTreeMap<String, usize>,
}

const SETS: [&str; 6] = ["deps", "limit", "release", "scan", "seat", "structure"];

static POLICY: LazyLock<Result<toml::Table, String>> =
    LazyLock::new(|| table("rules/suites/shape.toml").and_then(shaped));

static LIMIT: LazyLock<Result<toml::Table, String>> =
    LazyLock::new(|| table("rules/atoms/limit.toml"));

static RULES: LazyLock<Result<Rules, String>> = LazyLock::new(build);
pub fn prepare() -> Result<(), String> {
    policy()?;
    LIMIT.as_ref().map_err(Clone::clone)?;
    rules()?;
    Ok(())
}

pub fn policy() -> Result<&'static toml::Table, String> {
    POLICY.as_ref().map_err(Clone::clone)
}

pub fn rules() -> Result<&'static Rules, String> {
    RULES.as_ref().map_err(Clone::clone)
}

fn measures() -> &'static toml::Table {
    LIMIT
        .as_ref()
        .expect("limit access must follow depot preparation")
}

fn carried() -> &'static Rules {
    rules().expect("rule access must follow depot preparation")
}

fn shaped(table: toml::Table) -> Result<toml::Table, String> {
    for name in ["shape", "web"] {
        if table
            .get(name)
            .and_then(toml::Value::as_array)
            .is_none_or(Vec::is_empty)
        {
            return Err(format!("rules/suites/shape.toml must hold {name} rows"));
        }
    }
    Ok(table)
}

pub fn limits() -> BTreeMap<String, i64> {
    measured(measures())
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
        .unwrap_or_else(|| panic!("rules/atoms/limit.toml must hold a limit table"))
}

pub fn setting(section: &str, key: &str) -> bool {
    measures()
        .get(section)
        .and_then(|table| table.get(key))
        .and_then(toml::Value::as_bool)
        .unwrap_or_else(|| panic!("rules/atoms/limit.toml must hold {section}.{key}"))
}

pub fn read(name: &str) -> Result<toml::Table, String> {
    if !SETS.contains(&name) {
        return Err(format!("rule://{name} names no released set"));
    }
    table(&format!("rules/atoms/{name}.toml"))
}

pub fn current() -> &'static Rules {
    carried()
}

fn table(path: &str) -> Result<toml::Table, String> {
    let text = plumb::depot::rules()?.read(path)?;
    text.parse()
        .map_err(|error| format!("{path} does not parse: {error}"))
}

fn build() -> Result<Rules, String> {
    let structure = table("rules/atoms/structure.toml")?;
    let deps = table("rules/atoms/deps.toml")?;
    let release = table("rules/atoms/release.toml")?;
    let retired = deps
        .get("retired")
        .and_then(toml::Value::as_array)
        .map(|list| {
            list.iter()
                .map(|entry| {
                    let name = required(entry, "name", "rules/atoms/deps.toml retired entry")?;
                    let held = required(entry, "use", "rules/atoms/deps.toml retired entry")?;
                    Ok((name, held))
                })
                .collect::<Result<Vec<_>, String>>()
        })
        .transpose()?
        .unwrap_or_default();
    let blacklist = deps
        .get("blacklist")
        .and_then(toml::Value::as_array)
        .map(|list| {
            list.iter()
                .map(|value| {
                    value.as_str().map(str::to_string).ok_or_else(|| {
                        "rules/atoms/deps.toml blacklist must hold strings".to_string()
                    })
                })
                .collect::<Result<BTreeSet<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();
    Ok(Rules {
        dirs: members(&structure, "dir"),
        lanes: members(&structure, "lane"),
        retired,
        blacklist,
        release: Release {
            ceiling: ceiling(&release)?,
            permitted: counted(&release, "permitted"),
        },
    })
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

fn ceiling(doc: &toml::Table) -> Result<usize, String> {
    doc.get("ceiling")
        .and_then(toml::Value::as_integer)
        .and_then(|held| usize::try_from(held).ok())
        .ok_or_else(|| "rules/atoms/release.toml must name ceiling".to_string())
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

fn required(value: &toml::Value, key: &str, place: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(toml::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("{place} must name {key}"))
}
