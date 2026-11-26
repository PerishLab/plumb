use crate::shape::policy::Expected;
use crate::shape::section::{Origin, State};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub fn prepare(root: &Path) -> Result<(PathBuf, String), String> {
    Policy(root).prepare()
}

struct Policy<'a>(&'a Path);

impl Policy<'_> {
    fn prepare(&self) -> Result<(PathBuf, String), String> {
        match crate::shape::section::read(self.0, "ectropy.toml", "ectropy") {
            State::Unread(error) => Err(error),
            State::Held(Origin::File, table) => {
                Ok((self.0.join("ectropy.toml"), self.render(table)?))
            }
            State::Held(Origin::Section, table) => self.converge(table),
            State::Absent => match crate::shape::section::present(&self.0.join("plumb.toml")) {
                Ok(true) => self.converge(toml::Table::new()),
                Ok(false) => Ok((
                    self.0.join("ectropy.toml"),
                    self.render(toml::Table::new())?,
                )),
                Err(error) => Err(format!("cannot read plumb.toml: {error}")),
            },
        }
    }

    fn converge(&self, table: toml::Table) -> Result<(PathBuf, String), String> {
        if let Err(why) = crate::shape::policy::validate(&table) {
            return Err(format!("invalid [ectropy] in plumb.toml: {why}"));
        }
        let path = self.0.join("plumb.toml");
        let text = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
        Ok((path, self.patch(&text, table)?))
    }

    fn render(&self, table: toml::Table) -> Result<String, String> {
        let doc = self.reconcile(table);
        toml::to_string_pretty(&doc).map_err(|error| error.to_string())
    }

    fn patch(&self, text: &str, table: toml::Table) -> Result<String, String> {
        let lean = self.overlay(table);
        let mut doc = text
            .parse::<toml_edit::DocumentMut>()
            .map_err(|error| error.to_string())?;
        if lean.is_empty() {
            doc.remove("ectropy");
            return Ok(doc.to_string());
        }
        let target = seat(&doc);
        let body = toml::to_string_pretty(&lean).map_err(|error| error.to_string())?;
        let parsed = body
            .parse::<toml_edit::DocumentMut>()
            .map_err(|error| error.to_string())?;
        let mut section = parsed.as_table().clone();
        if section
            .iter()
            .all(|(_, item)| item.is_table() || item.is_array_of_tables())
        {
            section.set_implicit(true);
        }
        place(&mut section, target);
        doc["ectropy"] = toml_edit::Item::Table(section);
        Ok(doc.to_string())
    }

    fn overlay(&self, mut doc: toml::Table) -> toml::Table {
        let want = Expected::read(self.0);
        for owned in ["scan", "module", "limit", "comment", "word"] {
            doc.remove(owned);
        }
        prune(
            &mut doc,
            "grant",
            Some("test"),
            ("environment", &want.environment),
        );
        prune(&mut doc, "ban", None, ("style", &want.bans));
        doc
    }

    fn reconcile(&self, mut doc: toml::Table) -> toml::Table {
        let want = Expected::read(self.0);
        doc.insert(
            "scan".to_string(),
            toml::Value::Table(toml::Table::from_iter([
                ("include".to_string(), values(&want.include)),
                ("exclude".to_string(), values(&want.exclude)),
            ])),
        );
        doc.insert(
            "module".to_string(),
            toml::Value::Table(toml::Table::from_iter([(
                "roots".to_string(),
                values(&want.roots),
            )])),
        );
        doc.insert(
            "limit".to_string(),
            toml::Value::Table(
                crate::catalog::set::limits()
                    .into_iter()
                    .map(|(name, value)| (name, toml::Value::Integer(value)))
                    .collect(),
            ),
        );
        doc.insert(
            "comment".to_string(),
            toml::Value::Table(toml::Table::from_iter([(
                "allow".to_string(),
                toml::Value::Boolean(crate::catalog::set::setting("comment", "allow")),
            )])),
        );
        doc.insert(
            "word".to_string(),
            toml::Value::Table(toml::Table::from_iter([(
                "single".to_string(),
                toml::Value::Boolean(crate::catalog::set::setting("word", "single")),
            )])),
        );
        merge(&mut doc, ("grant", "test"), want.tests, false);
        merge(&mut doc, ("grant", "environment"), want.environment, true);
        merge(&mut doc, ("ban", "style"), want.bans, true);
        doc
    }
}

pub(super) fn prune(
    doc: &mut toml::Table,
    table: &str,
    dropped: Option<&str>,
    filter: (&str, &BTreeSet<String>),
) {
    let (name, covered) = filter;
    let entries = doc
        .remove(table)
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default();
    let mut kept = Vec::new();
    let mut extra = BTreeSet::new();
    for entry in entries {
        let kind = entry
            .as_table()
            .and_then(|held| held.get("syntax"))
            .and_then(toml::Value::as_str);
        if dropped.is_some() && kind == dropped {
            continue;
        }
        if kind == Some(name) {
            collect(&entry, covered, &mut extra);
        } else {
            kept.push(entry);
        }
    }
    if !extra.is_empty() {
        kept.push(toml::Value::Table(toml::Table::from_iter([
            ("syntax".to_string(), toml::Value::String(name.to_string())),
            ("paths".to_string(), values(&extra)),
        ])));
    }
    if !kept.is_empty() {
        doc.insert(table.to_string(), toml::Value::Array(kept));
    }
}

fn collect(entry: &toml::Value, covered: &BTreeSet<String>, extra: &mut BTreeSet<String>) {
    let Some(paths) = entry
        .as_table()
        .and_then(|held| held.get("paths"))
        .and_then(toml::Value::as_array)
    else {
        return;
    };
    extra.extend(
        paths
            .iter()
            .filter_map(toml::Value::as_str)
            .filter(|path| !covered.contains(*path))
            .map(str::to_string),
    );
}

fn merge(doc: &mut toml::Table, syntax: (&str, &str), mut paths: BTreeSet<String>, preserve: bool) {
    let (table, name) = syntax;
    let mut kept = Vec::new();
    let entries = doc
        .remove(table)
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default();
    for entry in entries {
        let held = entry.as_table();
        let matches = held
            .and_then(|value| value.get("syntax"))
            .and_then(toml::Value::as_str)
            == Some(name);
        if matches {
            if preserve
                && let Some(values) = held
                    .and_then(|value| value.get("paths"))
                    .and_then(toml::Value::as_array)
            {
                paths.extend(
                    values
                        .iter()
                        .filter_map(toml::Value::as_str)
                        .map(str::to_string),
                );
            }
        } else {
            kept.push(entry);
        }
    }
    if !paths.is_empty() {
        kept.push(toml::Value::Table(toml::Table::from_iter([
            ("syntax".to_string(), toml::Value::String(name.to_string())),
            ("paths".to_string(), values(&paths)),
        ])));
    }
    if !kept.is_empty() {
        doc.insert(table.to_string(), toml::Value::Array(kept));
    }
}

fn values(set: &BTreeSet<String>) -> toml::Value {
    toml::Value::Array(set.iter().cloned().map(toml::Value::String).collect())
}

fn seat(doc: &toml_edit::DocumentMut) -> isize {
    let mut existing = Vec::new();
    if let Some(item) = doc.get("ectropy") {
        bounds(item, &mut existing);
    }
    if let Some(min) = existing.into_iter().min() {
        return min;
    }
    let mut all = Vec::new();
    gather(doc.as_table(), &mut all);
    all.into_iter().max().map_or(0, |max| max + 1)
}

fn bounds(item: &toml_edit::Item, found: &mut Vec<isize>) {
    match item {
        toml_edit::Item::Table(table) => gather(table, found),
        toml_edit::Item::ArrayOfTables(list) => {
            for table in list.iter() {
                gather(table, found);
            }
        }
        _ => {}
    }
}

fn gather(table: &toml_edit::Table, found: &mut Vec<isize>) {
    if let Some(at) = table.position() {
        found.push(at);
    }
    for (_, item) in table.iter() {
        bounds(item, found);
    }
}

fn place(table: &mut toml_edit::Table, at: isize) {
    table.set_position(at);
    for (_, item) in table.iter_mut() {
        position(item, at);
    }
}

fn position(item: &mut toml_edit::Item, at: isize) {
    match item {
        toml_edit::Item::Table(inner) => place(inner, at),
        toml_edit::Item::ArrayOfTables(list) => {
            for inner in list.iter_mut() {
                place(inner, at);
            }
        }
        _ => {}
    }
}
