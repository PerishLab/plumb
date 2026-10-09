use super::cargo::{manifests, seat};
use std::path::{Path, PathBuf};

pub struct Workspace<'a>(pub &'a Path);

impl Workspace<'_> {
    pub fn read(&self) -> (Vec<(String, String)>, Vec<String>) {
        let mut found = Vec::new();
        let mut unread = Vec::new();
        let mut paths = manifests(self.0);
        paths.extend(self.members());
        paths.sort();
        paths.dedup();
        for path in paths {
            let held = seat(self.0, &path);
            match document(&path) {
                Ok(document) => found.extend(
                    plumb_cli::names(&document)
                        .into_iter()
                        .map(|name| (held.clone(), name)),
                ),
                Err(error) => unread.push(format!("cannot read Cargo manifest {held}: {error}")),
            }
        }
        (found, unread)
    }

    fn members(&self) -> Vec<PathBuf> {
        let declared = document(&self.0.join("Cargo.toml"))
            .ok()
            .and_then(|document| document.get("workspace")?.get("members").cloned());
        let mut found = Vec::new();
        for pattern in declared
            .iter()
            .flat_map(|held| held.as_array().into_iter().flatten())
        {
            let Some(pattern) = pattern.as_str() else {
                continue;
            };
            match pattern.strip_suffix("/*") {
                Some(parent) => found.extend(children(&self.0.join(parent))),
                None => found.push(self.0.join(pattern)),
            }
        }
        found
            .iter()
            .map(|path| path.join("Cargo.toml"))
            .filter(|path| path.is_file())
            .collect()
    }
}

fn document(path: &Path) -> Result<toml::Value, String> {
    std::fs::read_to_string(path)
        .map_err(|error| error.to_string())
        .and_then(|text| toml::from_str(&text).map_err(|error| error.to_string()))
}

fn children(parent: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(parent)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .collect()
}
