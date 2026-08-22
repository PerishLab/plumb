use std::collections::BTreeSet;
use std::path::Path;

mod render;
pub use render::render;

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

pub fn check(root: &Path, doc: &toml::Value) -> Vec<String> {
    let want = Expected::read(root);
    let policy = Policy(doc);
    let mut found = Vec::new();
    compare(
        "scan include",
        policy.list("scan", "include"),
        want.include,
        &mut found,
    );
    compare(
        "scan exclude",
        policy.list("scan", "exclude"),
        want.exclude,
        &mut found,
    );
    compare(
        "module roots",
        policy.list("module", "roots"),
        want.roots,
        &mut found,
    );
    policy.limits(&mut found);
    policy.setting(
        ("comment", "allow"),
        crate::catalog::set::setting("comment", "allow"),
        &mut found,
    );
    policy.setting(
        ("word", "single"),
        crate::catalog::set::setting("word", "single"),
        &mut found,
    );
    compare(
        "test grant",
        policy.syntax("grant", "test"),
        want.tests.clone(),
        &mut found,
    );
    require(
        "environment grant",
        policy.syntax("grant", "environment"),
        want.tests,
        &mut found,
    );
    require(
        "style ban",
        policy.syntax("ban", "style"),
        want.bans,
        &mut found,
    );
    found
}

struct Expected {
    include: BTreeSet<String>,
    exclude: BTreeSet<String>,
    roots: BTreeSet<String>,
    tests: BTreeSet<String>,
    bans: BTreeSet<String>,
}

impl Expected {
    fn read(root: &Path) -> Self {
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

struct Policy<'a>(&'a toml::Value);

impl Policy<'_> {
    fn limits(&self, found: &mut Vec<String>) {
        let factory = crate::catalog::set::factory();
        for (name, value) in crate::catalog::set::limits() {
            let seen = self
                .0
                .get("limit")
                .and_then(|table| table.get(&name))
                .and_then(toml::Value::as_integer);
            if seen != Some(value) && seen != factory.get(&name).copied() {
                found.push(format!("ectropy limit {name} must be {value}"));
            }
        }
    }

    fn setting(&self, key: (&str, &str), want: bool, found: &mut Vec<String>) {
        let seen = self
            .0
            .get(key.0)
            .and_then(|value| value.get(key.1))
            .and_then(toml::Value::as_bool);
        if seen != Some(want) {
            found.push(format!("ectropy {}.{} must be {want}", key.0, key.1));
        }
    }

    fn list(&self, table: &str, key: &str) -> BTreeSet<String> {
        self.0
            .get(table)
            .and_then(|value| value.get(key))
            .and_then(toml::Value::as_array)
            .map(|values| strings(values))
            .unwrap_or_default()
    }

    fn syntax(&self, table: &str, name: &str) -> BTreeSet<String> {
        let Some(entries) = self.0.get(table).and_then(toml::Value::as_array) else {
            return BTreeSet::new();
        };
        entries
            .iter()
            .filter(|entry| entry.get("syntax").and_then(toml::Value::as_str) == Some(name))
            .filter_map(|entry| entry.get("paths").and_then(toml::Value::as_array))
            .flat_map(|values| strings(values))
            .collect()
    }
}

fn strings(values: &[toml::Value]) -> BTreeSet<String> {
    values
        .iter()
        .filter_map(toml::Value::as_str)
        .map(str::to_string)
        .collect()
}

fn compare(name: &str, have: BTreeSet<String>, want: BTreeSet<String>, found: &mut Vec<String>) {
    require(name, have.clone(), want.clone(), found);
    for value in have.difference(&want) {
        found.push(format!("unexpected ectropy {name} {value}"));
    }
}

fn require(name: &str, have: BTreeSet<String>, want: BTreeSet<String>, found: &mut Vec<String>) {
    for value in want.difference(&have) {
        found.push(format!("missing ectropy {name} {value}"));
    }
}
