use std::collections::BTreeSet;
use std::path::Path;

use super::grammar::{atom, overlaps, seat};

#[derive(Clone)]
pub enum Config {
    Outside,
    Held(Vec<Binding>),
    Wrong(String),
    Blind(String),
}

#[derive(Clone)]
pub struct Binding {
    pub strategy: Strategy,
    pub name: Option<String>,
    pub sources: Vec<Source>,
    pub target: String,
}

#[derive(Clone)]
pub struct Source {
    pub path: String,
    pub seal: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Strategy {
    Agent,
    Architecture,
    Design,
    Brief,
}

impl Strategy {
    pub fn id(self) -> &'static str {
        match self {
            Self::Agent => "agent",
            Self::Architecture => "architecture",
            Self::Design => "design",
            Self::Brief => "brief",
        }
    }

    pub fn targets(self, name: Option<&str>) -> Vec<String> {
        match self {
            Self::Agent => vec!["AGENTS.md".into()],
            Self::Architecture => vec!["ARCHITECTURE.md".into()],
            Self::Design => vec!["DESIGN.md".into()],
            Self::Brief => ["SKILL.md", "PATHS.md", "SCENARIOS.md"]
                .into_iter()
                .map(|leaf| format!("skills/{}/{leaf}", name.unwrap_or_default()))
                .collect(),
        }
    }

    pub fn target(self, name: Option<&str>) -> String {
        match self {
            Self::Brief => format!("skills/{}", name.unwrap_or_default()),
            _ => self.targets(name).into_iter().next().unwrap_or_default(),
        }
    }
}

pub fn read(root: &Path) -> Config {
    let path = root.join("plumb.toml");
    if !path.exists() {
        return Config::Outside;
    }
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => return Config::Blind(error.to_string()),
    };
    let doc = match text.parse::<toml::Table>() {
        Ok(doc) => doc,
        Err(error) => return Config::Wrong(format!("cannot parse plumb.toml: {error}")),
    };
    if doc.contains_key("skill") || doc.contains_key("lock") {
        return Config::Wrong("plumb.toml contains retired document declarations".into());
    }
    let Some(value) = doc.get("document") else {
        return Config::Wrong("plumb.toml misses document declarations".into());
    };
    let Some(list) = value.as_array() else {
        return Config::Wrong("document must be an array of tables".into());
    };
    if list.is_empty() {
        return Config::Wrong("document must declare at least agent".into());
    }
    let mut held = Vec::new();
    for (rank, value) in list.iter().enumerate() {
        match binding(value) {
            Ok(binding) => held.push(binding),
            Err(error) => return Config::Wrong(format!("document #{} {error}", rank + 1)),
        }
    }
    if let Err(error) = unique(&held) {
        return Config::Wrong(error);
    }
    Config::Held(held)
}

fn binding(value: &toml::Value) -> Result<Binding, String> {
    let table = value
        .as_table()
        .ok_or_else(|| "must be a table".to_string())?;
    fields(table, &["strategy", "name", "source", "target-seal"])?;
    let strategy = match text(table, "strategy")?.as_str() {
        "agent" => Strategy::Agent,
        "architecture" => Strategy::Architecture,
        "design" => Strategy::Design,
        "brief" => Strategy::Brief,
        held => return Err(format!("has unknown strategy {held}")),
    };
    let name = table
        .get("name")
        .map(|value| {
            value
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| "name must be a string".to_string())
        })
        .transpose()?;
    match strategy {
        Strategy::Brief => {
            let name = name
                .as_deref()
                .ok_or_else(|| "brief misses name".to_string())?;
            if !atom(name) {
                return Err(format!("brief name {name} is not a lowercase atom"));
            }
        }
        _ if name.is_some() => return Err(format!("{} does not accept name", strategy.id())),
        _ => {}
    }
    let sources = source(table)?;
    if sources.is_empty() {
        return Err("has no source binding".into());
    }
    for left in 0..sources.len() {
        for right in left + 1..sources.len() {
            if overlaps(&sources[left].path, &sources[right].path) {
                return Err(format!(
                    "has overlapping source seats {} and {}",
                    sources[left].path, sources[right].path
                ));
            }
        }
    }
    let target = table
        .get("target-seal")
        .map(|value| {
            value
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| "target-seal must be a string".to_string())
        })
        .transpose()?
        .unwrap_or_default();
    Ok(Binding {
        strategy,
        name,
        sources,
        target,
    })
}

fn source(table: &toml::Table) -> Result<Vec<Source>, String> {
    let list = table
        .get("source")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| "source must be an array".to_string())?;
    let mut found = Vec::new();
    for value in list {
        let table = value
            .as_table()
            .ok_or_else(|| "source entries must be tables".to_string())?;
        fields(table, &["path", "seal"])?;
        let path = text(table, "path")?;
        if !seat(&path) {
            return Err(format!("source path {path} is not repository-relative"));
        }
        let seal = table
            .get("seal")
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_string)
                    .ok_or_else(|| "source seal must be a string".to_string())
            })
            .transpose()?
            .unwrap_or_default();
        found.push(Source { path, seal });
    }
    found.sort_by(|left, right| left.path.cmp(&right.path));
    if found.windows(2).any(|pair| pair[0].path == pair[1].path) {
        return Err("declares one source path more than once".into());
    }
    Ok(found)
}

fn unique(bindings: &[Binding]) -> Result<(), String> {
    let agents = bindings
        .iter()
        .filter(|held| held.strategy == Strategy::Agent)
        .count();
    if agents != 1 {
        return Err(format!(
            "document schema requires exactly one agent, got {agents}"
        ));
    }
    for strategy in [Strategy::Architecture, Strategy::Design] {
        let count = bindings
            .iter()
            .filter(|held| held.strategy == strategy)
            .count();
        if count > 1 {
            return Err(format!(
                "document strategy {} is declared {count} times",
                strategy.id()
            ));
        }
    }
    let mut names = BTreeSet::new();
    for binding in bindings
        .iter()
        .filter(|held| held.strategy == Strategy::Brief)
    {
        let name = binding.name.as_deref().unwrap_or_default();
        if !names.insert(name) {
            return Err(format!("brief {name} is declared more than once"));
        }
    }
    Ok(())
}

fn fields(table: &toml::Table, allowed: &[&str]) -> Result<(), String> {
    let extra = table
        .keys()
        .filter(|key| !allowed.contains(&key.as_str()))
        .map(String::as_str)
        .collect::<Vec<_>>();
    if extra.is_empty() {
        Ok(())
    } else {
        Err(format!("has unknown fields: {}", extra.join(", ")))
    }
}

fn text(table: &toml::Table, key: &str) -> Result<String, String> {
    table
        .get(key)
        .and_then(toml::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("misses string {key}"))
}
