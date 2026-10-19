use super::SOURCE;
use std::collections::{BTreeMap, BTreeSet};

type Id = (String, String, String);
type Graph = BTreeMap<Id, BTreeSet<Id>>;

pub(super) fn without(text: &str, moves: &super::Moves) -> Result<String, String> {
    let mut document = text
        .parse::<toml_edit::DocumentMut>()
        .map_err(|error| format!("invalid Cargo.lock: {error}"))?;
    let Some(packages) = document
        .get_mut("package")
        .and_then(toml_edit::Item::as_array_of_tables_mut)
    else {
        return Ok(text.into());
    };
    packages.retain(|table| {
        let field = |key: &str| {
            table
                .get(key)
                .and_then(toml_edit::Item::as_str)
                .unwrap_or_default()
                .to_string()
        };
        field("source") != SOURCE || !moves.contains(&(field("name"), field("version")))
    });
    Ok(document.to_string())
}

pub(super) fn forced(before: &str, after: &str) -> Result<(), String> {
    let old = read(before)?;
    let new = read(after)?;
    let change = Change {
        gone: reach(&old, &new),
        came: reach(&new, &old),
        old: &old,
        new: &new,
    };
    let mut faults = Vec::new();
    for id in old.keys().chain(new.keys()).collect::<BTreeSet<_>>() {
        if id.2 != SOURCE {
            change.judge(id, &mut faults);
        }
    }
    if faults.is_empty() {
        return Ok(());
    }
    Err(format!(
        "unforced third-party Cargo lock change, which no moved first-party package requires: {}",
        faults.join("; ")
    ))
}

struct Change<'a> {
    old: &'a Graph,
    new: &'a Graph,
    gone: BTreeSet<Id>,
    came: BTreeSet<Id>,
}

impl Change<'_> {
    fn judge(&self, id: &Id, faults: &mut Vec<String>) {
        match (self.old.get(id), self.new.get(id)) {
            (None, Some(_)) if !self.came.contains(id) => {
                faults.push(format!("{} {} added", id.0, id.1))
            }
            (Some(_), None) if !self.gone.contains(id) && !self.raised(id) => {
                faults.push(format!("{} {} removed", id.0, id.1))
            }
            (Some(_), Some(_)) => self.edges(id, faults),
            _ => {}
        }
    }

    fn fresh(&self, id: &Id) -> bool {
        self.came.contains(id) && !self.old.contains_key(id)
    }

    fn raised(&self, id: &Id) -> bool {
        self.came
            .iter()
            .any(|other| self.fresh(other) && slot(other, id))
    }

    fn paired(&self, lost: &Id, gained: &Id) -> bool {
        slot(lost, gained) && !self.new.contains_key(lost) && self.fresh(gained)
    }

    fn edges(&self, id: &Id, faults: &mut Vec<String>) {
        let edges = |graph: &Graph| graph.get(id).cloned().unwrap_or_default();
        let (old, new) = (edges(self.old), edges(self.new));
        let lost = old.difference(&new).filter(|target| target.2 != SOURCE);
        let gained = new
            .difference(&old)
            .filter(|target| target.2 != SOURCE)
            .collect::<Vec<_>>();
        let mut moved = BTreeSet::new();
        for target in lost {
            if gained.iter().any(|other| self.paired(target, other)) {
                moved.insert(&target.0);
                continue;
            }
            if let Some(other) = gained.iter().find(|other| other.0 == target.0) {
                moved.insert(&target.0);
                faults.push(format!(
                    "{} {} moved {} {} to {}",
                    id.0, id.1, target.0, target.1, other.1
                ));
            } else if !self.gone.contains(id) {
                faults.push(format!(
                    "{} {} dropped {} {}",
                    id.0, id.1, target.0, target.1
                ));
            }
        }
        for target in gained {
            if !moved.contains(&target.0) && !self.came.contains(id) {
                faults.push(format!("{} {} took {} {}", id.0, id.1, target.0, target.1));
            }
        }
    }
}

fn slot(left: &Id, right: &Id) -> bool {
    if left.0 != right.0 || left.2 != right.2 {
        return false;
    }
    match (
        semver::Version::parse(&left.1),
        semver::Version::parse(&right.1),
    ) {
        (Ok(one), Ok(two)) if one.major > 0 => one.major == two.major,
        (Ok(one), Ok(two)) if one.minor > 0 => two.major == 0 && one.minor == two.minor,
        (Ok(one), Ok(two)) => two.major == 0 && two.minor == 0 && one.patch == two.patch,
        _ => left.1 == right.1,
    }
}

fn reach(graph: &Graph, other: &Graph) -> BTreeSet<Id> {
    let mut stack = graph
        .keys()
        .filter(|id| id.2 == SOURCE && !other.contains_key(*id))
        .cloned()
        .collect::<Vec<_>>();
    let mut seen = BTreeSet::new();
    while let Some(id) = stack.pop() {
        if let Some(edges) = graph.get(&id)
            && seen.insert(id)
        {
            stack.extend(edges.iter().cloned());
        }
    }
    seen
}

fn read(text: &str) -> Result<Graph, String> {
    let value: toml::Value =
        toml::from_str(text).map_err(|error| format!("invalid Cargo.lock: {error}"))?;
    let items = value
        .get("package")
        .and_then(toml::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let ids = items.iter().map(identity).collect::<Result<Vec<_>, _>>()?;
    let mut graph = Graph::new();
    for (id, item) in ids.iter().zip(&items) {
        let mut edges = BTreeSet::new();
        for spec in item
            .get("dependencies")
            .and_then(toml::Value::as_array)
            .into_iter()
            .flatten()
        {
            let spec = spec
                .as_str()
                .ok_or("Cargo.lock dependency is not a string")?;
            edges.insert(target(&ids, spec)?);
        }
        graph.insert(id.clone(), edges);
    }
    Ok(graph)
}

fn identity(item: &toml::Value) -> Result<Id, String> {
    let field = |key: &str| item.get(key).and_then(toml::Value::as_str);
    let name = field("name").ok_or("locked Cargo package has no name")?;
    let version = field("version").ok_or("locked Cargo package has no version")?;
    let source = field("source").unwrap_or_default();
    Ok((name.into(), version.into(), source.into()))
}

fn target(ids: &[Id], spec: &str) -> Result<Id, String> {
    let mut parts = spec.splitn(3, ' ');
    let name = parts.next().unwrap_or_default();
    let version = parts.next();
    let source = parts.next().map(|source| source.trim_matches(['(', ')']));
    let found = ids
        .iter()
        .filter(|id| id.0 == name)
        .filter(|id| version.is_none_or(|version| id.1 == version))
        .filter(|id| source.is_none_or(|source| id.2 == source))
        .collect::<Vec<_>>();
    match found.as_slice() {
        [id] => Ok((*id).clone()),
        _ => Err(format!(
            "Cargo.lock dependency {spec} names no single package"
        )),
    }
}
