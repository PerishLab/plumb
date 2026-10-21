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

pub(crate) struct Seat<'a>(&'a Path);

impl<'a> Seat<'a> {
    pub(crate) fn new(root: &'a Path) -> Self {
        Self(root)
    }

    pub(crate) fn select(
        &self,
        snapshot: &plumb::snapshot::Snapshot,
        app: &str,
        lane: &plumb::lane::Name,
    ) -> Result<plumb::lane::declaration::Declaration, String> {
        let root = self.0;
        self.regular(snapshot, "plumb.toml")?;
        let entry = snapshot
            .entries()
            .iter()
            .find(|entry| entry.path() == "plumb.toml")
            .ok_or("lane status requires a tracked plumb.toml")?;
        let text = std::str::from_utf8(entry.bytes()).map_err(|_| "invalid lane declaration")?;
        let held = Self::parse(text)?.ok_or("repository declares no lane apps")?;
        self.judge(&held, snapshot)?;
        let source = super::qualification::Source::new(root, snapshot);
        for app in held.app.values() {
            source.judge(app)?;
        }
        let binding = held
            .app
            .get(app)
            .ok_or("requested lane app is not declared")?
            .binding
            .get(lane)
            .ok_or("requested lane is not declared; no default or fallback")?;
        let target = plumb::lane::Address::new(held.repository.clone(), app.into(), lane.clone())?;
        binding.declaration(target)
    }

    pub(crate) fn read(
        &self,
        snapshot: Result<&plumb::snapshot::Snapshot, &plumb::snapshot::Refusal>,
    ) -> Result<Vec<App>, String> {
        let root = self.0;
        let path = root.join("plumb.toml");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
        let Some(held) = Self::parse(&text)? else {
            return Ok(Vec::new());
        };
        let snapshot = snapshot.map_err(|error| format!("cannot judge preview source: {error}"))?;
        self.regular(snapshot, "plumb.toml")?;
        self.judge(&held, snapshot)?;
        Ok(held.app.into_values().collect())
    }

    fn parse(text: &str) -> Result<Option<Preview>, String> {
        let doc = text
            .parse::<toml::Table>()
            .map_err(|error| error.to_string())?;
        if doc.contains_key("preview") {
            return Err(
                "preview declarations are retired; declare app and bindings under lane".into(),
            );
        }
        let Some(preview) = doc.get("lane") else {
            return Ok(None);
        };
        let held: Preview = preview
            .clone()
            .try_into()
            .map_err(|error| error.to_string())?;
        if held.app.is_empty() {
            return Err("lane declaration needs at least one explicit app".into());
        }
        Ok(Some(held))
    }

    fn judge(&self, held: &Preview, snapshot: &plumb::snapshot::Snapshot) -> Result<(), String> {
        let mut paths = BTreeSet::new();
        let mut packages = BTreeSet::new();
        for (name, app) in &held.app {
            app.bindings(&held.repository, name)?;
            app.judge(self.0, snapshot)
                .map_err(|error| format!("preview app {name}: {error}"))?;
            if !paths.insert(app.path.clone()) || !packages.insert(app.package.clone()) {
                return Err(format!(
                    "preview app {name}: duplicate path or package identity"
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn regular(
        &self,
        snapshot: &plumb::snapshot::Snapshot,
        path: &str,
    ) -> Result<(), String> {
        let root = self.0;
        let tracked = snapshot
            .entries()
            .iter()
            .any(|entry| entry.path() == path && matches!(entry.mode(), "100644" | "100755"));
        if !tracked {
            return Err(format!("{path} must be a tracked regular file"));
        }
        self.ancestry(path)?;
        if !root.join(path).is_file() {
            return Err(format!("{path} must be a regular file"));
        }
        Ok(())
    }

    fn ancestry(&self, path: &str) -> Result<(), String> {
        let root = self.0;
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
        Seat::new(root).ancestry(&self.path)?;
        if !root.join(&self.path).is_dir() {
            return Err("path must name a repository directory".into());
        }
        for file in ["package.json", "wrangler.jsonc"] {
            Seat::new(root).regular(snapshot, &format!("{}/{file}", self.path))?;
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
