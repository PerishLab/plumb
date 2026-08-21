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
            }
        }
        if root.join("apps").is_dir() {
            held.web(root, "apps");
        }
        if root.join("packages").is_dir() {
            held.web(root, "packages");
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
        if root.join("skills").is_dir() {
            held.include.insert("skills/**/*.md".to_string());
            held.roots.insert("skills/*".to_string());
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

    fn web(&mut self, root: &Path, seat: &str) {
        let svelte = extension(&root.join(seat), "svelte");
        let tsx = !svelte || extension(&root.join(seat), "tsx");
        for suffix in ["ts", "css", "scss"] {
            self.include.insert(format!("{seat}/**/*.{suffix}"));
        }
        if svelte {
            self.include.insert(format!("{seat}/**/*.svelte"));
            self.exclude.insert("**/.svelte-kit/**".to_string());
        }
        if tsx {
            self.include.insert(format!("{seat}/**/*.tsx"));
        }
        self.roots.insert(format!("{seat}/*/src"));
        if seat == "packages" {
            self.roots.insert(format!("{seat}/*/lib"));
        }
        self.roots.insert(format!("{seat}/*/tests"));
        self.tests.insert(format!("{seat}/*/tests/**/*.test.ts"));
        if tsx {
            self.tests.insert(format!("{seat}/*/tests/**/*.test.tsx"));
        }
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
        for (name, value) in crate::catalog::set::limits() {
            let seen = self
                .0
                .get("limit")
                .and_then(|table| table.get(&name))
                .and_then(toml::Value::as_integer);
            if seen != Some(value) {
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
