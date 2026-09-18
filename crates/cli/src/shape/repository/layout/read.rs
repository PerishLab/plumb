use super::{Declared, Evidence, Group, Held, Read, Seat, affirm};
use plumb::snapshot::{Refusal, Snapshot};
use std::path::Path;

pub fn read(root: &Path, snapshot: Result<&Snapshot, &Refusal>) -> Read {
    let held = stated(root);
    let evidence = snapshot.ok().map(|snapshot| Evidence {
        snapshot: snapshot.clone(),
        repository: crate::anchor::Anchor(root).repo().unwrap_or_default(),
        affirmed: affirm::Held::read(root),
    });
    Read { held, evidence }
}

pub fn stated(root: &Path) -> Held {
    let path = root.join("plumb.toml");
    if !path.exists() {
        return Held::Outside;
    }
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => return Held::Wrong(format!("cannot read plumb.toml: {error}")),
    };
    let doc = match text.parse::<toml::Table>() {
        Ok(doc) => doc,
        Err(error) => return Held::Wrong(format!("cannot parse plumb.toml: {error}")),
    };
    match crate::shape::manifest::layered(doc) {
        Ok(doc) => declared(&doc),
        Err(error) => Held::Wrong(error),
    }
}

pub fn parse(text: &str) -> Held {
    match text.parse::<toml::Table>() {
        Ok(doc) => declared(&doc),
        Err(error) => Held::Wrong(format!("cannot parse plumb.toml: {error}")),
    }
}

fn declared(doc: &toml::Table) -> Held {
    let Some(value) = doc.get("layout") else {
        return Held::Absent;
    };
    match gather(value) {
        Ok(declared) => Held::Stated(declared),
        Err(error) => Held::Wrong(error),
    }
}

fn gather(value: &toml::Value) -> Result<Declared, String> {
    let table = value.as_table().ok_or("layout must be a table")?;
    let mut declared = Declared::default();
    for entry in Fields(table).listed("seat")? {
        declared.seats.push(seat(entry)?);
    }
    for entry in Fields(table).listed("file")? {
        declared.groups.push(group(entry)?);
    }
    if declared.seats.is_empty() && declared.groups.is_empty() {
        return Err("layout declares no seat and no file".to_string());
    }
    Ok(declared)
}

struct Fields<'a>(&'a toml::Table);

impl<'a> Fields<'a> {
    fn listed(&self, key: &str) -> Result<&'a [toml::Value], String> {
        match self.0.get(key) {
            None => Ok(&[]),
            Some(value) => value
                .as_array()
                .map(Vec::as_slice)
                .ok_or_else(|| format!("layout.{key} must be an array of tables")),
        }
    }

    fn retired(&self) -> Result<bool, String> {
        match self.0.get("kind").and_then(toml::Value::as_str) {
            None | Some("machine") => Ok(false),
            Some("retired") => Ok(true),
            Some(held) => Err(format!("layout kind {held} is unknown")),
        }
    }

    fn known(&self, held: &[&str]) -> Result<(), String> {
        for name in self.0.keys() {
            if !held.contains(&name.as_str()) {
                return Err(format!("layout carries no field called {name}"));
            }
        }
        Ok(())
    }

    fn text(&self, key: &str) -> Result<String, String> {
        self.0
            .get(key)
            .and_then(toml::Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| format!("a layout entry misses {key}"))
    }

    fn names(&self, key: &str) -> Result<Option<Vec<String>>, String> {
        let Some(value) = self.0.get(key) else {
            return Ok(None);
        };
        let list = value
            .as_array()
            .ok_or_else(|| format!("layout {key} must be an array"))?;
        let mut held = Vec::new();
        for entry in list {
            held.push(
                entry
                    .as_str()
                    .ok_or_else(|| format!("layout {key} holds a value that is not a name"))?
                    .to_string(),
            );
        }
        Ok(Some(held))
    }
}

fn seat(value: &toml::Value) -> Result<Seat, String> {
    let table = value.as_table().ok_or("a layout seat must be a table")?;
    let held = Fields(table);
    held.known(&["path", "anchor", "rule", "kind", "note", "depot"])?;
    let path = held.text("path")?;
    if path.is_empty() || path.starts_with('/') || path.contains("//") {
        return Err(format!("layout seat {path} is not a seat path"));
    }
    let depot = match table.get("depot") {
        None => None,
        Some(value) => Some(
            value
                .as_str()
                .ok_or("layout depot must be one seat name")?
                .to_string(),
        ),
    };
    if let Some(target) = &depot {
        if path.contains('*') {
            return Err(format!(
                "depot configuration seat {path} cannot be a pattern"
            ));
        }
        if target.is_empty()
            || !target
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            return Err(format!("layout depot target {target} is not one seat name"));
        }
        if held.retired()? {
            return Err(format!("retired layout seat {path} cannot feed the depot"));
        }
    }
    Ok(Seat {
        path,
        anchor: held.names("anchor")?,
        rule: held.names("rule")?.unwrap_or_default(),
        retired: held.retired()?,
        depot,
    })
}

fn group(value: &toml::Value) -> Result<Group, String> {
    let table = value
        .as_table()
        .ok_or("a layout file group must be a table")?;
    let seat = Fields(table);
    seat.known(&["name", "rule", "kind", "note"])?;
    let held = seat
        .names("name")?
        .ok_or("a layout file group names nothing")?;
    if held.is_empty() {
        return Err("a layout file group names nothing".to_string());
    }
    Ok(Group {
        names: held,
        rule: seat.names("rule")?.unwrap_or_default(),
        retired: seat.retired()?,
    })
}
