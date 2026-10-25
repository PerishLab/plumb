use super::super::{Package, compatible, process};
use crate::depot::v3;
use semver::Version;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub struct Sources {
    pub depot: String,
    pub index: String,
}

impl Default for Sources {
    fn default() -> Self {
        Self {
            depot: "https://depot.{product}.perish.uk".into(),
            index: "https://cargo.perish.uk".into(),
        }
    }
}

struct Move {
    ecosystem: String,
    name: String,
    old: Option<String>,
    new: Option<String>,
}

pub fn section(root: &Path, base: &[Package], head: &[Package], sources: &Sources) -> String {
    let rows = moves(base, head);
    let mut text = String::from("## Delta\n");
    if rows.is_empty() {
        text.push_str("No first-party package moved.\n");
        return text;
    }
    text.push_str("Machine-produced from the base and candidate locks. Class follows the plumb semver slot rule. Each crossed stable version links its Depot changelog MIGRATION.md pinned by generation; the Depot product is the package name before its first hyphen.\n\n");
    for row in rows {
        text.push_str(&row.render(root, sources));
    }
    text
}

fn moves(base: &[Package], head: &[Package]) -> Vec<Move> {
    let mut sides: BTreeMap<(&str, &str), [BTreeSet<&str>; 2]> = BTreeMap::new();
    for (side, packages) in [base, head].into_iter().enumerate() {
        for package in packages {
            let key = (package.ecosystem.as_str(), package.name.as_str());
            sides.entry(key).or_default()[side].insert(&package.version);
        }
    }
    let mut rows = Vec::new();
    for ((ecosystem, name), [before, after]) in sides {
        let gone = before.difference(&after).collect::<Vec<_>>();
        let came = after.difference(&before).collect::<Vec<_>>();
        let row = |old: Option<&&str>, new: Option<&&str>| Move {
            ecosystem: ecosystem.into(),
            name: name.into(),
            old: old.map(|text| text.to_string()),
            new: new.map(|text| text.to_string()),
        };
        if let ([old], [new]) = (gone.as_slice(), came.as_slice()) {
            rows.push(row(Some(old), Some(new)));
            continue;
        }
        rows.extend(gone.iter().map(|old| row(Some(old), None)));
        rows.extend(came.iter().map(|new| row(None, Some(new))));
    }
    rows
}

impl Move {
    fn render(&self, root: &Path, sources: &Sources) -> String {
        let (ecosystem, name) = (&self.ecosystem, &self.name);
        let (old, new) = match (&self.old, &self.new) {
            (Some(old), Some(new)) => (old, new),
            (Some(old), None) => return format!("- {ecosystem} {name} {old} removed\n"),
            (None, Some(new)) => {
                let line = reference(sources, name, new);
                return format!("- {ecosystem} {name} added {new}\n  - v{new} {line}\n");
            }
            (None, None) => return String::new(),
        };
        let class = if compatible(old, new) {
            "compatible"
        } else {
            "incompatible"
        };
        let mut text = format!("- {ecosystem} {name} {old} → {new} {class}\n");
        let (crossed, listed) = self.crossed(root, sources);
        if !listed {
            text.push_str("  - released versions unreadable; only the target version is linked\n");
        }
        for version in crossed {
            let line = reference(sources, name, &version);
            text.push_str(&format!("  - v{version} {line}\n"));
        }
        text
    }

    fn crossed(&self, root: &Path, sources: &Sources) -> (Vec<String>, bool) {
        let (Some(old), Some(new)) = (&self.old, &self.new) else {
            return (Vec::new(), true);
        };
        let (Ok(floor), Ok(ceiling)) = (Version::parse(old), Version::parse(new)) else {
            return (vec![new.clone()], false);
        };
        if ceiling <= floor {
            return (Vec::new(), true);
        }
        let mut found = released(root, &self.ecosystem, &self.name, sources)
            .unwrap_or_default()
            .into_iter()
            .filter(|version| *version > floor && *version <= ceiling)
            .collect::<Vec<_>>();
        found.sort();
        found.dedup();
        if found.last() != Some(&ceiling) {
            return (vec![new.clone()], false);
        }
        (found.iter().map(Version::to_string).collect(), true)
    }
}

fn released(
    root: &Path,
    ecosystem: &str,
    name: &str,
    sources: &Sources,
) -> Result<Vec<Version>, String> {
    if ecosystem == "cargo" {
        let url = format!("{}/{}", sources.index, super::route(name));
        let bytes = process::run(root, "probe", &super::probe(&url))?;
        return Ok(super::entries(&bytes)?
            .into_iter()
            .map(|entry| entry.version)
            .collect());
    }
    let argv = [
        "pnpm",
        "--config.ignore-pnpmfile=true",
        "view",
        name,
        "versions",
        "--json",
    ]
    .map(str::to_string);
    let bytes = process::run(root, "packages", &argv)?;
    let listed: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    let texts = match listed {
        serde_json::Value::Array(items) => items,
        single => vec![single],
    };
    Ok(texts
        .iter()
        .filter_map(serde_json::Value::as_str)
        .filter_map(|text| Version::parse(text).ok())
        .filter(|version| version.pre.is_empty())
        .collect())
}

fn reference(sources: &Sources, name: &str, version: &str) -> String {
    let bare = name.strip_prefix("@perishlab/").unwrap_or(name);
    let product = bare.split('-').next().unwrap_or(bare);
    let source = sources.depot.replace("{product}", product);
    let tag = format!("v{version}");
    let query = v3::Query {
        source: &source,
        product,
        channel: "stable",
        version: &tag,
        kind: v3::Kind::Changelog,
    };
    let found = match v3::Generation::latest(query) {
        Ok(Some(found)) => found,
        Ok(None) => return format!("changelog: missing ({product} Depot)"),
        Err(error) => return format!("changelog: missing ({product} Depot: {error})"),
    };
    let generation = &found.pointer.generation;
    let objects = &found.manifest.objects;
    let note = objects
        .iter()
        .find(|object| object.path == "en/MIGRATION.md")
        .or_else(|| {
            objects
                .iter()
                .find(|object| object.path.ends_with("MIGRATION.md"))
        });
    match note {
        Some(object) => format!(
            "MIGRATION {}objects/{} (generation {generation})",
            found.url(),
            object.path
        ),
        None => format!("changelog: missing MIGRATION.md (generation {generation})"),
    }
}
