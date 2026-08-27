use crate::catalog::set;
use crate::shape::workflow::Key;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

#[derive(Default)]
pub struct Tree {
    root: PathBuf,
    revision: Option<String>,
    leaves: BTreeMap<String, String>,
}
pub struct Git<'a>(&'a Path);

#[derive(Clone, Serialize)]
pub struct Project {
    pub path: String,
    pub omit: Vec<String>,
}

#[derive(Default)]
pub struct Projects(BTreeMap<String, Vec<Project>>);

impl Tree {
    pub fn read(root: &Path, rev: Option<&str>) -> Result<Self, String> {
        let listed = match rev {
            Some(rev) => Git::new(root).listing(&["ls-tree", "-r", "-z", rev])?,
            None => Git::new(root).listing(&["ls-files", "--stage", "-z"])?,
        };
        let mut held = BTreeMap::new();
        for record in listed.split('\0').filter(|record| !record.is_empty()) {
            let (meta, path) = record
                .split_once('\t')
                .ok_or_else(|| "git emitted a malformed entry".to_string())?;
            let mut fields = meta.split(' ');
            let mode = fields.next().unwrap_or_default();
            let oid = match rev {
                Some(_) => fields.nth(1).unwrap_or_default(),
                None => fields.next().unwrap_or_default(),
            };
            if mode.is_empty() || oid.is_empty() {
                return Err("git emitted a malformed entry".to_string());
            }
            held.insert(path.to_string(), format!("{mode} {oid}"));
        }
        Ok(Self {
            root: root.to_path_buf(),
            revision: rev.map(str::to_string),
            leaves: held,
        })
    }

    fn under(&self, roots: &[String]) -> Vec<(&String, &String)> {
        self.leaves
            .iter()
            .filter(|(path, _)| roots.iter().any(|root| covers(root, path)))
            .collect()
    }

    pub fn digest(&self, key: &Key) -> String {
        self.projected(key, &[])
            .expect("an unprojected tree digest cannot fail")
    }

    pub fn projected(&self, key: &Key, projects: &[Project]) -> Result<String, String> {
        for project in projects {
            if !key.paths.iter().any(|root| covers(root, &project.path)) {
                return Err(format!(
                    "{} projects {}, outside its input roots",
                    key.name(),
                    project.path
                ));
            }
            if !self.leaves.contains_key(&project.path) {
                return Err(format!(
                    "{} projects an absent leaf {}",
                    key.name(),
                    project.path
                ));
            }
        }
        let mut sponge = Sha256::new();
        sponge.update(key.name().as_bytes());
        sponge.update([0]);
        for root in &key.roots {
            sponge.update(root.as_bytes());
            sponge.update([0]);
        }
        sponge.update([1]);
        sponge.update(set::current().release.forge.as_bytes());
        sponge.update([0]);
        if !projects.is_empty() {
            sponge.update([2]);
            for project in projects {
                sponge.update(project.path.as_bytes());
                sponge.update([0]);
                for pointer in &project.omit {
                    sponge.update(pointer.as_bytes());
                    sponge.update([0]);
                }
                sponge.update([1]);
            }
        }
        for (path, meta) in self.under(&key.paths) {
            sponge.update(path.as_bytes());
            sponge.update([0]);
            if let Some(project) = projects.iter().find(|project| project.path == *path) {
                sponge.update(self.project(project)?.as_bytes());
            } else {
                sponge.update(meta.as_bytes());
            }
            sponge.update([0]);
        }
        Ok(format!("{:x}", sponge.finalize()))
    }

    pub fn covered(&self, key: &Key) -> usize {
        self.under(&key.paths).len()
    }

    pub fn has(&self, path: &str) -> bool {
        self.leaves.contains_key(path)
    }

    fn project(&self, project: &Project) -> Result<String, String> {
        let object = match &self.revision {
            Some(revision) => format!("{revision}:{}", project.path),
            None => format!(":{}", project.path),
        };
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args(["show", &object])
            .output()
            .map_err(|error| format!("cannot execute git: {error}"))?;
        if !output.status.success() {
            return Err(format!("cannot read projected leaf {}", project.path));
        }
        let mut value: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|error| format!("cannot project {} as JSON: {error}", project.path))?;
        for pointer in &project.omit {
            let selected = value.pointer_mut(pointer).ok_or_else(|| {
                format!("projected leaf {} has no pointer {pointer}", project.path)
            })?;
            *selected = serde_json::Value::Null;
        }
        let mode = self
            .leaves
            .get(&project.path)
            .and_then(|meta| meta.split_once(' '))
            .map(|(mode, _)| mode)
            .unwrap_or_default();
        serde_json::to_string(&value)
            .map(|value| format!("{mode} {value}"))
            .map_err(|error| format!("cannot encode projected leaf {}: {error}", project.path))
    }
}

impl Projects {
    pub fn parse(entries: &[String]) -> Result<Self, String> {
        let mut held: BTreeMap<String, BTreeMap<String, Vec<String>>> = BTreeMap::new();
        for entry in entries {
            let (action, source) = entry
                .split_once('=')
                .ok_or_else(|| format!("project entry {entry:?} must be ACTION=PATH#POINTER"))?;
            let (path, pointer) = source
                .split_once('#')
                .ok_or_else(|| format!("project entry {entry:?} must be ACTION=PATH#POINTER"))?;
            if action.trim().is_empty() || !relative(path) || !pointer.starts_with('/') {
                return Err(format!(
                    "project entry {entry:?} must name an action, relative path, and JSON pointer"
                ));
            }
            let pointers = held
                .entry(action.to_string())
                .or_default()
                .entry(path.to_string())
                .or_default();
            if pointers.iter().any(|held| held == pointer) {
                return Err(format!(
                    "project entry {entry:?} is declared more than once"
                ));
            }
            pointers.push(pointer.to_string());
            pointers.sort();
        }
        Ok(Self(
            held.into_iter()
                .map(|(action, paths)| {
                    let projects = paths
                        .into_iter()
                        .map(|(path, omit)| Project { path, omit })
                        .collect();
                    (action, projects)
                })
                .collect(),
        ))
    }

    pub fn action(&self, name: &str) -> Vec<Project> {
        self.0.get(name).cloned().unwrap_or_default()
    }

    pub fn declare(&self, keys: &mut Vec<Key>) -> Result<(), String> {
        for action in self.0.keys() {
            if keys.iter().any(|key| key.name() == *action) {
                continue;
            }
            let (lane, output) = action
                .split_once('/')
                .filter(|(lane, output)| !lane.is_empty() && !output.is_empty())
                .ok_or_else(|| format!("project action {action:?} must be LANE/OUTPUT"))?;
            let mut segments = vec![lane.to_string()];
            segments.extend(output.split('.').map(str::to_string));
            if !set::current().lanes.contains(lane)
                || segments.iter().any(|segment| segment.is_empty())
            {
                return Err(format!(
                    "project action {action:?} is not a declared lane output"
                ));
            }
            keys.push(Key {
                segments,
                roots: vec!["*".to_string()],
                paths: vec!["*".to_string()],
            });
        }
        Ok(())
    }
}

fn covers(root: &str, path: &str) -> bool {
    root == "*" || path == root || path.starts_with(&format!("{root}/"))
}

fn relative(path: &str) -> bool {
    !path.is_empty()
        && Path::new(path)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

impl<'a> Git<'a> {
    pub fn new(root: &'a Path) -> Self {
        Self(root)
    }

    pub fn history(&self, rev: &str) -> Result<Vec<String>, String> {
        let range = format!("{rev}..HEAD");
        let listed = self.listing(&["rev-list", "--reverse", &range])?;
        Ok(listed
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect())
    }

    pub fn revision(&self, rev: &str) -> Result<String, String> {
        Ok(self
            .listing(&["rev-parse", "--verify", rev])?
            .trim()
            .to_string())
    }

    pub fn file(&self, rev: &str, path: &str) -> Result<Option<String>, String> {
        let object = format!("{rev}:{path}");
        let output = Command::new("git")
            .arg("-C")
            .arg(self.0)
            .args(["show", &object])
            .output()
            .map_err(|error| format!("cannot execute git: {error}"))?;
        if output.status.success() {
            return String::from_utf8(output.stdout)
                .map(Some)
                .map_err(|_| "git emitted non-UTF-8 output".to_string());
        }
        let error = String::from_utf8_lossy(&output.stderr);
        if error.contains("does not exist") || error.contains("exists on disk, but not in") {
            return Ok(None);
        }
        Err(error.trim().to_string())
    }

    fn listing(&self, args: &[&str]) -> Result<String, String> {
        let output = Command::new("git")
            .arg("-C")
            .arg(self.0)
            .args(args)
            .output()
            .map_err(|error| format!("cannot execute git: {error}"))?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
        }
        String::from_utf8(output.stdout).map_err(|_| "git emitted non-UTF-8 output".to_string())
    }
}
