use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Preview {
    repository: String,
    app: BTreeMap<String, App>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct App {
    pub(super) path: String,
    pub(super) package: String,
    pub(super) mapping: super::mapping::Mapping,
    binding: BTreeMap<plumb::lane::Name, super::binding::Binding>,
}

pub(crate) fn read(
    root: &Path,
    snapshot: Result<&plumb::snapshot::Snapshot, &plumb::snapshot::Refusal>,
) -> Result<Vec<App>, String> {
    let path = root.join("plumb.toml");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    let doc = text
        .parse::<toml::Table>()
        .map_err(|error| error.to_string())?;
    if doc.contains_key("preview") {
        return Err("preview declarations are retired; declare app and bindings under lane".into());
    }
    let Some(preview) = doc.get("lane") else {
        return Ok(Vec::new());
    };
    let held: Preview = preview
        .clone()
        .try_into()
        .map_err(|error| error.to_string())?;
    if held.app.is_empty() {
        return Err("lane declaration needs at least one explicit app".into());
    }
    let snapshot = snapshot.map_err(|error| format!("cannot judge preview source: {error}"))?;
    regular(root, snapshot, "plumb.toml")?;
    let mut paths = BTreeSet::new();
    let mut packages = BTreeSet::new();
    let mut apps = Vec::new();
    for (name, app) in held.app {
        app.bindings(&held.repository, &name)?;
        app.judge(root, snapshot)
            .map_err(|error| format!("preview app {name}: {error}"))?;
        if !paths.insert(app.path.clone()) || !packages.insert(app.package.clone()) {
            return Err(format!(
                "preview app {name}: duplicate path or package identity"
            ));
        }
        apps.push(app);
    }
    Ok(apps)
}

impl App {
    fn bindings(&self, repository: &str, name: &str) -> Result<(), String> {
        if self.binding.is_empty() {
            return Err(format!("lane app {name}: needs explicit lane bindings"));
        }
        for (lane, binding) in &self.binding {
            let target = plumb::lane::Address::new(repository.into(), name.into(), lane.clone())?;
            binding.judge(target)?;
        }
        Ok(())
    }

    fn judge(&self, root: &Path, snapshot: &plumb::snapshot::Snapshot) -> Result<(), String> {
        self.mapping.judge()?;
        if !literal(&self.package) {
            return Err("package must be a literal package selector".into());
        }
        directory(&self.path)?;
        ancestry(root, &self.path)?;
        if !root.join(&self.path).is_dir() {
            return Err("path must name a repository directory".into());
        }
        for file in ["package.json", "wrangler.jsonc"] {
            regular(root, snapshot, &format!("{}/{file}", self.path))?;
        }
        Ok(())
    }
}

fn directory(path: &str) -> Result<(), String> {
    let platform = Path::new(path).is_absolute() || path.as_bytes().get(1) == Some(&b':');
    let segments = path.split('/').any(|part| matches!(part, "" | "." | ".."));
    if platform || segments || path.contains('\\') {
        return Err("path must be a normalized relative POSIX directory".into());
    }
    Ok(())
}

fn literal(package: &str) -> bool {
    let name = if let Some(scoped) = package.strip_prefix('@') {
        let Some((scope, name)) = scoped.split_once('/') else {
            return false;
        };
        if !token(scope, false) {
            return false;
        }
        name
    } else {
        package
    };
    token(name, true)
}

fn token(value: &str, dotted: bool) -> bool {
    value.starts_with(|byte: char| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        && value.bytes().all(|byte| match byte {
            b'a'..=b'z' | b'0'..=b'9' | b'-' => true,
            b'.' | b'_' => dotted,
            _ => false,
        })
}

pub(super) fn regular(
    root: &Path,
    snapshot: &plumb::snapshot::Snapshot,
    path: &str,
) -> Result<(), String> {
    let tracked = snapshot
        .entries()
        .iter()
        .any(|entry| entry.path() == path && matches!(entry.mode(), "100644" | "100755"));
    if !tracked {
        return Err(format!("{path} must be a tracked regular file"));
    }
    ancestry(root, path)?;
    if !root.join(path).is_file() {
        return Err(format!("{path} must be a regular file"));
    }
    Ok(())
}

fn ancestry(root: &Path, path: &str) -> Result<(), String> {
    let mut target = root.to_path_buf();
    for part in path.split('/') {
        target.push(part);
        let metadata = std::fs::symlink_metadata(&target)
            .map_err(|error| format!("cannot read {path}: {error}"))?;
        if metadata.file_type().is_symlink() {
            return Err(format!("{path} traverses a symlink"));
        }
    }
    Ok(())
}
