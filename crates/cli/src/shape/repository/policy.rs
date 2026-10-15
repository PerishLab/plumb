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
    pub environment: BTreeSet<String>,
    pub bans: BTreeSet<String>,
}

impl Expected {
    pub(crate) fn read(root: &Path) -> Self {
        Self::from(
            root,
            crate::catalog::set::policy().expect("policy access must follow depot preparation"),
        )
    }

    fn from(root: &Path, policy: &toml::Table) -> Self {
        let mut held = Self {
            include: BTreeSet::new(),
            exclude: BTreeSet::new(),
            roots: BTreeSet::new(),
            tests: BTreeSet::new(),
            environment: BTreeSet::new(),
            bans: BTreeSet::new(),
        };
        for row in rows(policy, "shape") {
            let table = row
                .as_table()
                .unwrap_or_else(|| panic!("rules/suites/shape.toml rows must be tables"));
            if list(table, "when")
                .iter()
                .all(|path| root.join(path).is_dir())
            {
                held.select(root, table, "use");
                if table.contains_key("tsx")
                    && list(table, "when").iter().any(|path| {
                        let base = root.join(path);
                        extension(&base, "ts") || extension(&base, "tsx")
                    })
                {
                    held.select(root, table, "tsx");
                }
            }
        }
        for row in rows(policy, "web") {
            let table = row
                .as_table()
                .unwrap_or_else(|| panic!("rules/suites/shape.toml web rows must be tables"));
            let seat = table
                .get("seat")
                .and_then(toml::Value::as_str)
                .unwrap_or_else(|| panic!("rules/suites/shape.toml web rows must name a seat"));
            let base = root.join(seat);
            if !base.is_dir() {
                continue;
            }
            held.select(root, table, "use");
            let svelte = extension(&base, "svelte");
            if svelte {
                held.select(root, table, "svelte");
            }
            if !svelte || extension(&base, "tsx") {
                held.select(root, table, "tsx");
            }
        }
        held
    }

    fn select(&mut self, root: &Path, table: &toml::Table, key: &str) {
        for held in list(table, key) {
            let set = crate::catalog::member::scan(&held)
                .unwrap_or_else(|error| panic!("rules/suites/shape.toml names {held}: {error}"));
            self.apply(&set, "include", Part::Include);
            self.apply(&set, "exclude", Part::Exclude);
            self.apply(&set, "roots", Part::Root);
            self.roots.extend(Tree { root }.derive(&set));
            self.apply(&set, "tests", Part::Test);
            self.reads(&set, "tests");
            self.apply(&set, "bans", Part::Ban);
        }
    }

    fn reads(&mut self, table: &toml::Table, fallback: &str) {
        let key = table.contains_key("environment").then_some("environment");
        self.apply(table, key.unwrap_or(fallback), Part::Environment);
    }

    fn apply(&mut self, table: &toml::Table, key: &str, part: Part) {
        let target = match part {
            Part::Include => &mut self.include,
            Part::Exclude => &mut self.exclude,
            Part::Root => &mut self.roots,
            Part::Test => &mut self.tests,
            Part::Environment => &mut self.environment,
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
    Environment,
    Ban,
}

fn rows<'a>(policy: &'a toml::Table, name: &str) -> &'a [toml::Value] {
    policy
        .get(name)
        .and_then(toml::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_else(|| panic!("rules/suites/shape.toml must hold {name} rows"))
}

fn list(table: &toml::Table, key: &str) -> BTreeSet<String> {
    match table.get(key) {
        Some(value) => value
            .as_array()
            .unwrap_or_else(|| panic!("a {key} list is a list"))
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .unwrap_or_else(|| panic!("a {key} list holds strings"))
                    .to_string()
            })
            .collect(),
        None => BTreeSet::new(),
    }
}

struct Tree<'a> {
    root: &'a Path,
}

impl Tree<'_> {
    fn derive(&self, set: &toml::Table) -> BTreeSet<String> {
        let statics = list(set, "roots");
        let suffixes: BTreeSet<String> = list(set, "include")
            .iter()
            .filter_map(|glob| glob.rsplit_once("*.").map(|(_, tail)| tail.to_string()))
            .collect();
        list(set, "derive")
            .iter()
            .flat_map(|pattern| self.expand(pattern))
            .flat_map(|parent| self.children(&parent))
            .filter(|child| !statics.iter().any(|glob| matches(glob, child)))
            .filter(|child| {
                suffixes
                    .iter()
                    .any(|suffix| extension(&self.root.join(child), suffix))
            })
            .collect()
    }

    fn children(&self, parent: &str) -> Vec<String> {
        let Ok(entries) = std::fs::read_dir(self.root.join(parent)) else {
            return Vec::new();
        };
        let mut names: Vec<String> = entries
            .flatten()
            .filter(|entry| entry.path().is_dir())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .map(|name| join(parent, &name))
            .collect();
        names.sort();
        names
    }

    fn expand(&self, pattern: &str) -> Vec<String> {
        pattern
            .split('/')
            .fold(vec![String::new()], |held, segment| {
                held.iter()
                    .flat_map(|prefix| self.step(prefix, segment))
                    .collect()
            })
    }

    fn step(&self, prefix: &str, segment: &str) -> Vec<String> {
        if segment == "*" {
            return self.children(prefix);
        }
        let path = join(prefix, segment);
        if self.root.join(&path).is_dir() {
            vec![path]
        } else {
            Vec::new()
        }
    }
}

fn join(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{prefix}/{name}")
    }
}

fn matches(glob: &str, path: &str) -> bool {
    let left: Vec<&str> = glob.split('/').collect();
    let right: Vec<&str> = path.split('/').collect();
    left.len() == right.len()
        && left
            .iter()
            .zip(&right)
            .all(|(want, have)| *want == "*" || want == have)
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
