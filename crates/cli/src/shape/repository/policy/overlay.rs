use super::schema;
use super::section::{Origin, State};
use super::{Evidence, Expected};
use std::collections::BTreeSet;
use std::path::Path;

pub(crate) struct Resolved {
    pub laws: bool,
    pub unread: Option<String>,
    pub document: Option<toml::Value>,
    pub policy: Option<Evidence>,
}

impl Resolved {
    fn unread(why: String) -> Self {
        Self {
            laws: true,
            unread: Some(why),
            document: None,
            policy: None,
        }
    }

    fn held(document: toml::Value, policy: Evidence) -> Self {
        Self {
            laws: true,
            unread: None,
            document: Some(document),
            policy: Some(policy),
        }
    }
}

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

pub(crate) fn resolve(root: &Path) -> Resolved {
    match super::section::read(root, "ectropy.toml", "ectropy") {
        State::Unread(why) => Resolved::unread(why),
        State::Held(Origin::File, table) => {
            let document = toml::Value::Table(table);
            let policy = Evidence::read(root, document.clone());
            Resolved::held(document, policy)
        }
        State::Held(Origin::Section, table) => converge(root, table),
        State::Absent => match super::section::present(&root.join("plumb.toml")) {
            Ok(true) => converge(root, toml::Table::new()),
            Ok(false) => Resolved {
                laws: false,
                unread: None,
                document: None,
                policy: None,
            },
            Err(why) => Resolved::unread(format!("cannot read plumb.toml: {why}")),
        },
    }
}

fn converge(root: &Path, overlay: toml::Table) -> Resolved {
    if let Err(why) = schema::validate(&overlay) {
        return Resolved::unread(format!("invalid [ectropy] in plumb.toml: {why}"));
    }
    let expected = Expected::read(root);
    let document = toml::Value::Table(apply(defaults(&expected), overlay));
    let policy = Evidence {
        actual: document.clone(),
        expected,
    };
    Resolved::held(document, policy)
}

fn defaults(expected: &Expected) -> toml::Table {
    let mut doc = toml::Table::from_iter([
        (
            "scan".to_string(),
            pair(
                "include",
                array(&expected.include),
                "exclude",
                array(&expected.exclude),
            ),
        ),
        (
            "module".to_string(),
            single("roots", array(&expected.roots)),
        ),
        (
            "limit".to_string(),
            toml::Value::Table(
                crate::catalog::set::limits()
                    .into_iter()
                    .map(|(name, value)| (name, toml::Value::Integer(value)))
                    .collect(),
            ),
        ),
        (
            "comment".to_string(),
            single(
                "allow",
                toml::Value::Boolean(crate::catalog::set::setting("comment", "allow")),
            ),
        ),
        (
            "word".to_string(),
            single(
                "single",
                toml::Value::Boolean(crate::catalog::set::setting("word", "single")),
            ),
        ),
    ]);
    let mut grants = Vec::new();
    if !expected.tests.is_empty() {
        grants.push(syntax("test", &expected.tests));
    }
    if !expected.environment.is_empty() {
        grants.push(syntax("environment", &expected.environment));
    }
    if !grants.is_empty() {
        doc.insert("grant".to_string(), toml::Value::Array(grants));
    }
    if !expected.bans.is_empty() {
        doc.insert(
            "ban".to_string(),
            toml::Value::Array(vec![syntax("style", &expected.bans)]),
        );
    }
    doc
}

fn apply(mut base: toml::Table, mut overlay: toml::Table) -> toml::Table {
    let mut target = Target(&mut base);
    if let Some(toml::Value::Table(mut extra)) = overlay.remove("scan") {
        target.union("scan", "include", extra.remove("include"));
        target.union("scan", "exclude", extra.remove("exclude"));
    }
    if let Some(toml::Value::Table(mut extra)) = overlay.remove("module") {
        target.union("module", "roots", extra.remove("roots"));
    }
    for section in ["limit", "comment"] {
        target.replace(section, overlay.remove(section));
    }
    for key in ["grant", "ban", "boundary"] {
        target.extend(key, overlay.remove(key));
    }
    base
}

struct Target<'a>(&'a mut toml::Table);

impl Target<'_> {
    fn union(&mut self, section: &str, key: &str, added: Option<toml::Value>) {
        let Some(toml::Value::Array(items)) = added else {
            return;
        };
        let Some(target) = self
            .0
            .get_mut(section)
            .and_then(toml::Value::as_table_mut)
            .and_then(|held| held.get_mut(key))
            .and_then(toml::Value::as_array_mut)
        else {
            return;
        };
        for item in items {
            if !target.contains(&item) {
                target.push(item);
            }
        }
    }

    fn replace(&mut self, section: &str, added: Option<toml::Value>) {
        let Some(toml::Value::Table(extra)) = added else {
            return;
        };
        let Some(target) = self.0.get_mut(section).and_then(toml::Value::as_table_mut) else {
            return;
        };
        for (key, value) in extra {
            target.insert(key, value);
        }
    }

    fn extend(&mut self, key: &str, added: Option<toml::Value>) {
        let Some(toml::Value::Array(items)) = added else {
            return;
        };
        match self.0.get_mut(key).and_then(toml::Value::as_array_mut) {
            Some(target) => target.extend(items),
            None => {
                self.0.insert(key.to_string(), toml::Value::Array(items));
            }
        }
    }
}

fn array(set: &BTreeSet<String>) -> toml::Value {
    toml::Value::Array(set.iter().cloned().map(toml::Value::String).collect())
}

fn single(key: &str, value: toml::Value) -> toml::Value {
    toml::Value::Table(toml::Table::from_iter([(key.to_string(), value)]))
}

fn pair(first: &str, left: toml::Value, second: &str, right: toml::Value) -> toml::Value {
    toml::Value::Table(toml::Table::from_iter([
        (first.to_string(), left),
        (second.to_string(), right),
    ]))
}

fn syntax(name: &str, paths: &BTreeSet<String>) -> toml::Value {
    pair(
        "syntax",
        toml::Value::String(name.to_string()),
        "paths",
        array(paths),
    )
}
