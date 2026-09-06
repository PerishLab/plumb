use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

const HELD: &str = "plumb-release-identity";

pub struct Seat<'a>(pub &'a Path);

impl Seat<'_> {
    pub fn blind(&self, product: &str, built: Option<&str>) -> Option<String> {
        let built = built.filter(|_| product == "plumb")?;
        let head = self.reference("HEAD")?;
        if head == built || !self.ancestor(built, &head) || self.settled(built, &head) {
            return None;
        }
        Some(format!(
            "running Plumb was built at {built}, before this Plumb tree at {head}; rebuild Plumb from the tree before asking Doctor to judge it"
        ))
    }

    fn reference(&self, name: &str) -> Option<String> {
        let output = self.git(["rev-parse", name]).output().ok()?;
        output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn ancestor(&self, old: &str, new: &str) -> bool {
        self.git(["merge-base", "--is-ancestor", old, new])
            .status()
            .is_ok_and(|status| status.success())
    }

    fn settled(&self, built: &str, head: &str) -> bool {
        let Some([base, release]) = self.parents(head) else {
            return false;
        };
        release == built && self.tree(&base) == self.tree(head) && self.equivalent(&base, &release)
    }

    fn equivalent(&self, base: &str, release: &str) -> bool {
        let (Some(before), Some(after)) = (self.leaves(base), self.leaves(release)) else {
            return false;
        };
        let paths = before.keys().chain(after.keys()).collect::<BTreeSet<_>>();
        paths.into_iter().all(|path| {
            if path.starts_with(".plumb/releases/") && path.ends_with("/datum.toml") {
                return true;
            }
            let (Some(left), Some(right)) = (before.get(path), after.get(path)) else {
                return false;
            };
            if left == right {
                return true;
            }
            matches!(
                (
                    fingerprint(true, (self.0, Some(&before.revision), false), path, left),
                    fingerprint(true, (self.0, Some(&after.revision), false), path, right),
                ),
                (Ok(left), Ok(right)) if left == right
            )
        })
    }

    fn leaves(&self, revision: &str) -> Option<Leaves> {
        let output = self.git(["ls-tree", "-r", "-z", revision]).output().ok()?;
        if !output.status.success() {
            return None;
        }
        let mut leaves = BTreeMap::new();
        for record in String::from_utf8_lossy(&output.stdout)
            .split('\0')
            .filter(|record| !record.is_empty())
        {
            let (meta, path) = record.split_once('\t')?;
            let mut fields = meta.split_whitespace();
            let mode = fields.next()?;
            let kind = fields.next()?;
            let object = fields.next()?;
            if kind != "blob" {
                return None;
            }
            leaves.insert(path.to_string(), format!("{mode} {object}"));
        }
        Some(Leaves {
            revision: revision.to_string(),
            entries: leaves,
        })
    }

    fn parents(&self, commit: &str) -> Option<[String; 2]> {
        let output = self
            .git(["show", "-s", "--format=%P", commit])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let body = String::from_utf8_lossy(&output.stdout);
        let mut parents = body.split_whitespace().map(str::to_string);
        let pair = [parents.next()?, parents.next()?];
        parents.next().is_none().then_some(pair)
    }

    fn tree(&self, commit: &str) -> Option<String> {
        self.reference(&format!("{commit}^{{tree}}"))
    }

    fn git<const N: usize>(&self, args: [&str; N]) -> Command {
        let mut command = Command::new("git");
        command.arg("-C").arg(self.0).args(args);
        command
    }
}

struct Leaves {
    revision: String,
    entries: BTreeMap<String, String>,
}

impl Leaves {
    fn keys(&self) -> impl Iterator<Item = &String> {
        self.entries.keys()
    }

    fn get(&self, path: &str) -> Option<&String> {
        self.entries.get(path)
    }
}

pub(crate) fn fingerprint(
    guard: bool,
    source: (&Path, Option<&str>, bool),
    path: &str,
    meta: &str,
) -> Result<Option<String>, String> {
    if !guard && path.starts_with(".forgejo/") {
        return Ok(None);
    }
    if !guard || source.2 {
        return Ok(Some(meta.to_string()));
    }
    if path.starts_with(".plumb/releases/") && path.ends_with("/datum.toml") {
        return Ok(None);
    }
    let normalized = match path {
        "Cargo.lock" => cargo(&object(source.0, source.1, path)?, true)?,
        path if path.ends_with("Cargo.toml") => cargo(&object(source.0, source.1, path)?, false)?,
        path if path.ends_with("package.json") => package(&object(source.0, source.1, path)?)?,
        path if path.ends_with("Chart.yaml") => chart(&object(source.0, source.1, path)?)?,
        _ => return Ok(Some(meta.to_string())),
    };
    let mode = meta.split_once(' ').map_or("", |(mode, _)| mode);
    Ok(Some(format!("{mode} {normalized}")))
}

fn object(root: &Path, revision: Option<&str>, path: &str) -> Result<Vec<u8>, String> {
    let object = revision.map_or_else(|| format!(":{path}"), |held| format!("{held}:{path}"));
    let output = plumb::config::detached("git")
        .arg("-C")
        .arg(root)
        .args(["show", &object])
        .output()
        .map_err(|error| format!("cannot read release identity leaf {path}: {error}"))?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(format!("cannot read release identity leaf {path}"))
    }
}

fn cargo(bytes: &[u8], lock: bool) -> Result<String, String> {
    let table: toml::Table = std::str::from_utf8(bytes)
        .map_err(|error| format!("release Cargo identity is not UTF-8: {error}"))?
        .parse()
        .map_err(|error| format!("cannot parse release Cargo identity: {error}"))?;
    let mut doc = toml::Value::Table(table);
    if lock {
        let packages = doc
            .get_mut("package")
            .and_then(toml::Value::as_array_mut)
            .ok_or_else(|| "release Cargo lock has no package array".to_string())?;
        for package in packages {
            let Some(table) = package.as_table_mut() else {
                continue;
            };
            if !table.contains_key("source") && table.contains_key("version") {
                table.insert("version".into(), toml::Value::String(HELD.into()));
            }
        }
    } else {
        manifest(&mut doc);
    }
    serde_json::to_string(&doc).map_err(|error| error.to_string())
}

fn manifest(value: &mut toml::Value) {
    let Some(table) = value.as_table_mut() else {
        return;
    };
    if let Some(package) = table.get_mut("package").and_then(toml::Value::as_table_mut)
        && package.contains_key("version")
    {
        package.insert("version".into(), toml::Value::String(HELD.into()));
    }
    if let Some(package) = table
        .get_mut("workspace")
        .and_then(toml::Value::as_table_mut)
        .and_then(|workspace| workspace.get_mut("package"))
        .and_then(toml::Value::as_table_mut)
        && package.contains_key("version")
    {
        package.insert("version".into(), toml::Value::String(HELD.into()));
    }
    if table.contains_key("path") && table.contains_key("version") {
        table.insert("version".into(), toml::Value::String(HELD.into()));
    }
    for (_, child) in table.iter_mut() {
        manifest(child);
    }
}

fn package(bytes: &[u8]) -> Result<String, String> {
    let mut doc: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|error| format!("cannot parse release package identity: {error}"))?;
    if let Some(version) = doc.get_mut("version") {
        *version = serde_json::Value::String(HELD.into());
    }
    serde_json::to_string(&doc).map_err(|error| error.to_string())
}

fn chart(bytes: &[u8]) -> Result<String, String> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| format!("release chart identity is not UTF-8: {error}"))?;
    Ok(text
        .lines()
        .map(|line| {
            if line.starts_with("version:") || line.starts_with("appVersion:") {
                line.split_once(':')
                    .map_or_else(|| line.to_string(), |(name, _)| format!("{name}: {HELD}"))
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n"))
}
