use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub fn check(root: &Path, doc: &toml::Value) -> Vec<String> {
    let want = Expected::read(root);
    let mut found = Vec::new();
    compare(
        "scan include",
        list(doc, "scan", "include"),
        want.include,
        &mut found,
    );
    compare(
        "scan exclude",
        list(doc, "scan", "exclude"),
        want.exclude,
        &mut found,
    );
    compare(
        "module roots",
        list(doc, "module", "roots"),
        want.roots,
        &mut found,
    );
    limits(doc, &mut found);
    setting(doc, "comment", "allow", false, &mut found);
    setting(doc, "word", "single", true, &mut found);
    compare(
        "test grant",
        syntax(doc, "grant", "test"),
        want.tests.clone(),
        &mut found,
    );
    require(
        "environment grant",
        syntax(doc, "grant", "environment"),
        want.tests,
        &mut found,
    );
    require(
        "style ban",
        syntax(doc, "ban", "style"),
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
        if root.join("crates").is_dir() {
            held.rust("crates/**/*.rs", "crates/*");
        }
        if root.join("app").is_dir() {
            held.include.insert("app/src/**/*.rs".to_string());
            held.include.insert("app/tests/**/*.rs".to_string());
            held.roots.insert("app/src".to_string());
            held.roots.insert("app/tests".to_string());
            held.tests.insert("app/tests/**/*.rs".to_string());
            if root.join(".runseal").is_dir() {
                held.include.insert(".runseal/**".to_string());
                held.roots.insert(".runseal".to_string());
                held.tests.insert(".runseal/**/*.test.ts".to_string());
            }
        }
        if root.join("apps").is_dir() {
            held.web("apps");
        }
        if root.join("packages").is_dir() {
            held.web("packages");
        }
        if root.join("lib").is_dir() {
            held.include.insert("lib/**/*.ts".to_string());
            held.include.insert("lib/**/*.tsx".to_string());
            held.roots.insert("lib".to_string());
        }
        if root.join("tests").is_dir() {
            held.include.insert("tests/**/*.ts".to_string());
            held.include.insert("tests/**/*.tsx".to_string());
            held.roots.insert("tests".to_string());
            held.tests.insert("tests/**/*.test.ts".to_string());
            held.tests.insert("tests/**/*.test.tsx".to_string());
        }
        if root.join("docs").is_dir() {
            held.include.insert("docs/**/*.md".to_string());
            held.roots.insert("docs".to_string());
        }
        if root.join("crates").is_dir() || root.join("app").is_dir() {
            held.exclude.insert("**/target/**".to_string());
        }
        if ["apps", "packages", "lib"]
            .iter()
            .any(|seat| root.join(seat).is_dir())
        {
            held.exclude.insert("**/node_modules/**".to_string());
        }
        if ["apps", "packages"]
            .iter()
            .any(|seat| root.join(seat).is_dir())
        {
            held.exclude.insert("**/dist/**".to_string());
        }
        if root.join("apps/web/src/lib/components").is_dir() {
            held.bans
                .insert("apps/web/src/lib/components/**".to_string());
        }
        held
    }

    fn rust(&mut self, include: &str, root: &str) {
        self.include.insert(include.to_string());
        self.roots.insert(format!("{root}/src"));
        self.roots.insert(format!("{root}/tests"));
        self.tests.insert(format!("{root}/tests/**/*.rs"));
    }

    fn web(&mut self, seat: &str) {
        for suffix in ["ts", "tsx", "css", "scss"] {
            self.include.insert(format!("{seat}/**/*.{suffix}"));
        }
        self.roots.insert(format!("{seat}/*/src"));
        if seat == "packages" {
            self.roots.insert(format!("{seat}/*/lib"));
        }
        self.roots.insert(format!("{seat}/*/tests"));
        self.tests.insert(format!("{seat}/*/tests/**/*.test.ts"));
        self.tests.insert(format!("{seat}/*/tests/**/*.test.tsx"));
    }
}

fn limits(doc: &toml::Value, found: &mut Vec<String>) {
    let want = BTreeMap::from([
        ("block", 4),
        ("fanout", 10),
        ("file", 300),
        ("markup", 8),
        ("param", 4),
        ("path", 4),
    ]);
    for (name, value) in want {
        let seen = doc
            .get("limit")
            .and_then(|table| table.get(name))
            .and_then(toml::Value::as_integer);
        if seen != Some(value) {
            found.push(format!("ectropy limit {name} must be {value}"));
        }
    }
}

fn setting(doc: &toml::Value, table: &str, key: &str, want: bool, found: &mut Vec<String>) {
    let seen = doc
        .get(table)
        .and_then(|value| value.get(key))
        .and_then(toml::Value::as_bool);
    if seen != Some(want) {
        found.push(format!("ectropy {table}.{key} must be {want}"));
    }
}

fn list(doc: &toml::Value, table: &str, key: &str) -> BTreeSet<String> {
    doc.get(table)
        .and_then(|value| value.get(key))
        .and_then(toml::Value::as_array)
        .map(|values| strings(values))
        .unwrap_or_default()
}

fn syntax(doc: &toml::Value, table: &str, name: &str) -> BTreeSet<String> {
    let Some(entries) = doc.get(table).and_then(toml::Value::as_array) else {
        return BTreeSet::new();
    };
    entries
        .iter()
        .filter(|entry| entry.get("syntax").and_then(toml::Value::as_str) == Some(name))
        .filter_map(|entry| entry.get("paths").and_then(toml::Value::as_array))
        .flat_map(|values| strings(values))
        .collect()
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
