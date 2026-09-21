use std::collections::BTreeSet;
use toml::{Table, Value};

pub const DEFAULTS: &str = "rules/plumb.toml";

pub fn layered(project: Table) -> Result<(Table, Vec<String>), String> {
    let mut overrides = Vec::new();
    match defaults()? {
        Some(held) => Ok((merge(held, project, "", &mut overrides), overrides)),
        None => Ok((project, overrides)),
    }
}

fn defaults() -> Result<Option<Table>, String> {
    let Ok(rules) = plumb::depot::rules() else {
        return Ok(None);
    };
    if !rules.objects().iter().any(|object| object.path == DEFAULTS) {
        return Ok(None);
    }
    rules
        .read(DEFAULTS)?
        .parse()
        .map(Some)
        .map_err(|error| format!("cannot parse Depot {DEFAULTS}: {error}"))
}

fn taken(before: Value, stated: Value, path: String, overrides: &mut Vec<String>) -> Value {
    if before != stated {
        overrides.push(path);
    }
    stated
}

pub fn merge(mut held: Table, stated: Table, at: &str, overrides: &mut Vec<String>) -> Table {
    for (key, value) in stated {
        let path = if at.is_empty() {
            key.clone()
        } else {
            format!("{at}.{key}")
        };
        let merged = match (held.remove(&key), value, identity(&path)) {
            (Some(Value::Table(held)), Value::Table(stated), _) => {
                Value::Table(merge(held, stated, &path, overrides))
            }
            (Some(Value::Array(held)), Value::Array(stated), Some(field)) => {
                Value::Array(keyed(held, stated, field, overrides))
            }
            (Some(before), stated, _) => taken(before, stated, path, overrides),
            (None, stated, _) => stated,
        };
        held.insert(key, merged);
    }
    held
}

fn identity(path: &str) -> Option<&'static str> {
    match path {
        "layout.seat" => Some("path"),
        "layout.file" => Some("name"),
        _ => None,
    }
}

fn keyed(
    held: Vec<Value>,
    stated: Vec<Value>,
    field: &str,
    overrides: &mut Vec<String>,
) -> Vec<Value> {
    let claimed: BTreeSet<String> = stated
        .iter()
        .flat_map(|entry| names(entry, field))
        .collect();
    for entry in &held {
        for name in names(entry, field) {
            if claimed.contains(&name) {
                overrides.push(format!("{field} {name}"));
            }
        }
    }
    let mut kept: Vec<Value> = held
        .into_iter()
        .filter_map(|entry| released(entry, field, &claimed))
        .collect();
    kept.extend(stated);
    kept
}

fn names(entry: &Value, field: &str) -> Vec<String> {
    match entry.get(field) {
        Some(Value::String(name)) => vec![name.clone()],
        Some(Value::Array(list)) => list
            .iter()
            .filter_map(|name| name.as_str().map(str::to_string))
            .collect(),
        _ => Vec::new(),
    }
}

fn released(mut entry: Value, field: &str, claimed: &BTreeSet<String>) -> Option<Value> {
    let table = entry.as_table_mut()?;
    match table.get(field) {
        Some(Value::String(name)) if claimed.contains(name) => None,
        Some(Value::Array(list)) => {
            let left: Vec<Value> = list
                .iter()
                .filter(|name| name.as_str().is_none_or(|name| !claimed.contains(name)))
                .cloned()
                .collect();
            if left.is_empty() {
                return None;
            }
            table.insert(field.to_string(), Value::Array(left));
            Some(entry)
        }
        _ => Some(entry),
    }
}

#[cfg(test)]
mod tests {
    use super::merge;
    use toml::Table;

    fn table(text: &str) -> Table {
        text.parse().expect("fixture table")
    }

    #[test]
    fn scalars() {
        let held = table("[release]\nproduct = \"base\"\nskill = true\n[other]\nkept = 1\n");
        let stated = table("[release]\nproduct = \"probe\"\ntargets = [\"a\"]\n");
        let merged = merge(held, stated, "", &mut Vec::new());
        assert_eq!(merged["release"]["product"].as_str(), Some("probe"));
        assert_eq!(merged["release"]["skill"].as_bool(), Some(true));
        assert_eq!(merged["other"]["kept"].as_integer(), Some(1));
    }

    #[test]
    fn seats() {
        let held = table(
            "[[layout.seat]]\npath = \"skills/*\"\nrule = [\"a\"]\n[[layout.seat]]\npath = \"charts/*\"\n",
        );
        let stated = table(
            "[[layout.seat]]\npath = \"skills/*\"\nrule = [\"b\"]\n[[layout.seat]]\npath = \"crates/*\"\n",
        );
        let merged = merge(held, stated, "", &mut Vec::new());
        let seats = merged["layout"]["seat"].as_array().expect("seats");
        let paths: Vec<_> = seats
            .iter()
            .map(|seat| seat["path"].as_str().unwrap())
            .collect();
        assert_eq!(paths, ["charts/*", "skills/*", "crates/*"]);
        assert_eq!(seats[1]["rule"][0].as_str(), Some("b"));
    }

    #[test]
    fn groups() {
        let held = table(
            "[[layout.file]]\nname = [\"LICENSE\", \".gitignore\"]\n[[layout.file]]\nname = [\"AGENTS.md\"]\nrule = [\"affirmed\"]\n",
        );
        let stated = table("[[layout.file]]\nname = [\"AGENTS.md\"]\n");
        let merged = merge(held, stated, "", &mut Vec::new());
        let groups = merged["layout"]["file"].as_array().expect("groups");
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0]["name"].as_array().map(Vec::len), Some(2));
        assert!(groups[1].get("rule").is_none());
    }

    #[test]
    fn arrays() {
        let held = table("[release]\ntargets = [\"a\", \"b\"]\n");
        let stated = table("[release]\ntargets = [\"c\"]\n");
        let merged = merge(held, stated, "", &mut Vec::new());
        assert_eq!(
            merged["release"]["targets"].as_array().map(Vec::len),
            Some(1)
        );
    }

    #[test]
    fn overrides() {
        let held = table(
            "[[layout.seat]]\npath = \"skills/*\"\n[[layout.file]]\nname = [\"LICENSE\", \"AGENTS.md\"]\n[release]\nskill = true\n",
        );
        let stated = table(
            "[[layout.seat]]\npath = \"skills/*\"\nrule = [\"b\"]\n[[layout.file]]\nname = [\"AGENTS.md\"]\n[release]\nskill = false\n",
        );
        let mut overrides = Vec::new();
        merge(held, stated, "", &mut overrides);
        overrides.sort();
        assert_eq!(
            overrides,
            ["name AGENTS.md", "path skills/*", "release.skill"]
        );
    }
}
