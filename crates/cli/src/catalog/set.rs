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

const SETS: [&str; 5] = ["deps", "release", "seat", "structure", "workflow"];

static POLICY: LazyLock<Result<toml::Table, String>> =
    LazyLock::new(|| table("rules/policy.toml").and_then(shaped));

static RULES: LazyLock<Result<Rules, String>> = LazyLock::new(build);
pub fn prepare() -> Result<(), String> {
    policy()?;
    rules()?;
    Ok(())
}

pub fn policy() -> Result<&'static toml::Table, String> {
    POLICY.as_ref().map_err(Clone::clone)
}

pub fn rules() -> Result<&'static Rules, String> {
    RULES.as_ref().map_err(Clone::clone)
}

fn held() -> &'static toml::Table {
    policy().expect("policy access must follow depot preparation")
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
            return Err(format!("rules/policy.toml must hold {name} rows"));
        }
    }
    Ok(table)
}

pub fn limits() -> BTreeMap<String, i64> {
    measured(held())
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
    held()
        .get(section)
        .and_then(|table| table.get(key))
        .and_then(toml::Value::as_bool)
        .unwrap_or_else(|| panic!("rules/policy.toml must hold {section}.{key}"))
}

pub fn read(name: &str) -> Result<toml::Table, String> {
    if !SETS.contains(&name) {
        return Err(format!("rule://{name} names no released set"));
    }
    table(&format!("rules/{name}.toml"))
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
    let structure = table("rules/structure.toml")?;
    let workflow = table("rules/workflow.toml")?;
    let deps = table("rules/deps.toml")?;
    let release = table("rules/release.toml")?;
    let suites = workflow
        .get("suite")
        .and_then(toml::Value::as_table)
        .map(|table| {
            table
                .iter()
                .map(|(name, value)| Ok((name.clone(), paths(value)?)))
                .collect::<Result<BTreeMap<_, _>, String>>()
        })
        .transpose()?
        .unwrap_or_default();
    let retired = deps
        .get("retired")
        .and_then(toml::Value::as_array)
        .map(|list| {
            list.iter()
                .map(|entry| {
                    let name = required(entry, "name", "rules/deps.toml retired entry")?;
                    let held = required(entry, "use", "rules/deps.toml retired entry")?;
                    Ok((name, held))
                })
                .collect::<Result<Vec<_>, String>>()
        })
        .transpose()?
        .unwrap_or_default();
    let stable = deps
        .get("stable")
        .ok_or_else(|| "rules/deps.toml must name stable authorities".to_string())?;
    let cargo = stable
        .get("cargo")
        .ok_or_else(|| "rules/deps.toml must name stable.cargo".to_string())?;
    let blacklist =
        deps.get("blacklist")
            .and_then(toml::Value::as_array)
            .map(|list| {
                list.iter()
                    .map(|value| {
                        value.as_str().map(str::to_string).ok_or_else(|| {
                            "rules/deps.toml blacklist must hold strings".to_string()
                        })
                    })
                    .collect::<Result<BTreeSet<_>, _>>()
            })
            .transpose()?
            .unwrap_or_default();
    let forge = release
        .get("forge")
        .and_then(|table| table.get("image"))
        .and_then(toml::Value::as_str)
        .ok_or_else(|| "rules/release.toml must name forge.image".to_string())?
        .to_string();
    Ok(Rules {
        suites,
        dirs: members(&structure, "dir"),
        lanes: members(&structure, "lane"),
        retired,
        blacklist,
        stable: Stable {
            cargo: Cargo {
                registry: required(cargo, "registry", "rules/deps.toml stable.cargo")?,
                index: required(cargo, "index", "rules/deps.toml stable.cargo")?,
            },
        },
        release: Release {
            ceiling: ceiling(&release)?,
            permitted: counted(&release, "permitted"),
            exercised: counted(&release, "exercised"),
            forge,
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
        .ok_or_else(|| "rules/release.toml must name ceiling".to_string())
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

fn paths(value: &toml::Value) -> Result<Vec<String>, String> {
    value
        .as_array()
        .ok_or_else(|| "rules/workflow.toml suite must hold paths".to_string())?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| "rules/workflow.toml suite paths must be strings".to_string())
        })
        .collect()
}
