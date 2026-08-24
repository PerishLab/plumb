use std::collections::BTreeSet;
use std::path::Path;

pub fn bounds(doc: Option<&toml::Value>) -> Vec<String> {
    let mut found = Vec::new();
    let Some(list) = doc
        .and_then(|value| value.get("boundary"))
        .and_then(toml::Value::as_array)
    else {
        return found;
    };
    for edge in list {
        let Some(paths) = edge.get("paths").and_then(toml::Value::as_array) else {
            continue;
        };
        for path in paths.iter().filter_map(toml::Value::as_str) {
            if !path.contains('*') {
                found.push(path.to_string());
            }
        }
    }
    found
}

pub(crate) struct Evidence {
    pub actual: toml::Value,
    pub expected: Expected,
}

impl Evidence {
    pub(crate) fn read(root: &Path, actual: toml::Value) -> Self {
        Self {
            actual,
            expected: Expected::read(root),
        }
    }
}

pub(crate) struct Expected {
    pub include: BTreeSet<String>,
    pub exclude: BTreeSet<String>,
    pub roots: BTreeSet<String>,
    pub tests: BTreeSet<String>,
    pub bans: BTreeSet<String>,
}

impl Expected {
    pub(crate) fn read(root: &Path) -> Self {
        let mut held = Self {
            include: BTreeSet::new(),
            exclude: BTreeSet::new(),
            roots: BTreeSet::new(),
            tests: BTreeSet::new(),
            bans: BTreeSet::new(),
        };
        for row in rows("shape") {
            let table = row
                .as_table()
                .unwrap_or_else(|| panic!("rules/policy.toml shape rows must be tables"));
            if list(table, "when")
                .iter()
                .all(|path| root.join(path).is_dir())
            {
                held.apply(table, "include", Part::Include);
                held.apply(table, "exclude", Part::Exclude);
                held.apply(table, "roots", Part::Root);
                held.apply(table, "tests", Part::Test);
                held.apply(table, "bans", Part::Ban);
            }
        }
        for row in rows("web") {
            let table = row
                .as_table()
                .unwrap_or_else(|| panic!("rules/policy.toml web rows must be tables"));
            let seat = table
                .get("seat")
                .and_then(toml::Value::as_str)
                .unwrap_or_else(|| panic!("rules/policy.toml web rows must name a seat"));
            let base = root.join(seat);
            if !base.is_dir() {
                continue;
            }
            held.apply(table, "include", Part::Include);
            held.apply(table, "exclude", Part::Exclude);
            held.apply(table, "roots", Part::Root);
            held.apply(table, "tests", Part::Test);
            let svelte = extension(&base, "svelte");
            let tsx = extension(&base, "tsx");
            if svelte {
                held.apply(table, "svelte-include", Part::Include);
                held.apply(table, "svelte-exclude", Part::Exclude);
            }
            if !svelte || tsx {
                held.apply(table, "tsx-include", Part::Include);
                held.apply(table, "tsx-tests", Part::Test);
            }
        }
        held
    }

    fn apply(&mut self, table: &toml::Table, key: &str, part: Part) {
        let target = match part {
            Part::Include => &mut self.include,
            Part::Exclude => &mut self.exclude,
            Part::Root => &mut self.roots,
            Part::Test => &mut self.tests,
            Part::Ban => &mut self.bans,
        };
        target.extend(list(table, key));
    }
}

enum Part {
    Include,
    Exclude,
    Root,
    Test,
    Ban,
}

fn rows(name: &str) -> &'static [toml::Value] {
    crate::catalog::set::POLICY
        .get(name)
        .and_then(toml::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_else(|| panic!("rules/policy.toml must hold {name} rows"))
}

fn list(table: &toml::Table, key: &str) -> BTreeSet<String> {
    match table.get(key) {
        Some(value) => value
            .as_array()
            .unwrap_or_else(|| panic!("rules/policy.toml {key} must be a list"))
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .unwrap_or_else(|| panic!("rules/policy.toml {key} must hold strings"))
                    .to_string()
            })
            .collect(),
        None => BTreeSet::new(),
    }
}

fn extension(root: &Path, suffix: &str) -> bool {
    let Ok(entries) = std::fs::read_dir(root) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let path = entry.path();
        if path.is_dir() {
            extension(&path, suffix)
        } else {
            path.extension().and_then(|value| value.to_str()) == Some(suffix)
        }
    })
}
