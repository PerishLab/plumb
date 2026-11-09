mod app;
mod document;
mod entry;
mod views;

use crate::shape::web::Setup;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct App {
    pub(super) path: PathBuf,
    pub(super) setup: Setup,
    pub(super) mode: super::Mode,
    pub(super) config: String,
}

#[derive(Serialize)]
pub(super) struct Entry {
    pub(super) source: String,
    pub(super) view: String,
    pub(super) target: String,
    pub(super) output: String,
    pub(super) digest: String,
}

pub(super) struct Source<'a>(pub(super) &'a Path);

impl Source<'_> {
    pub(super) fn apps(&self, mode: super::Mode) -> Result<Vec<App>, String> {
        let root = self.0;
        let mut paths = vec![PathBuf::new()];
        let directory = root.join("apps");
        if directory.is_dir() {
            for entry in std::fs::read_dir(directory).map_err(|error| error.to_string())? {
                let entry = entry.map_err(|error| error.to_string())?;
                if entry
                    .file_type()
                    .map_err(|error| error.to_string())?
                    .is_symlink()
                {
                    return Err("symlinked app boundary is unqualified".into());
                }
                if entry
                    .file_type()
                    .map_err(|error| error.to_string())?
                    .is_dir()
                {
                    paths.push(Path::new("apps").join(entry.file_name()));
                }
            }
        }
        paths.sort();
        let mut found = Vec::new();
        for path in paths {
            if let Some(app) = app::read(root, &path, mode)? {
                found.push(app)
            }
        }
        Ok(found)
    }

    pub(super) fn views(
        &self,
        app: &App,
        sources: &[(PathBuf, Vec<super::svelte::Mark>)],
    ) -> Result<(), String> {
        views::verify(self.0, app, sources)
    }

    pub(super) fn built(&self, app: &App) -> Result<Entry, String> {
        views::built(self.0, app)
    }

    pub(super) fn files(&self) -> Result<Vec<PathBuf>, String> {
        Ok(self
            .leaves()?
            .into_iter()
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "svelte")
            })
            .collect())
    }

    fn leaves(&self) -> Result<Vec<PathBuf>, String> {
        let root = self.0;
        let mut found = Vec::new();
        if !root.is_dir() {
            return Err(format!("missing brand source directory {}", root.display()));
        }
        for entry in std::fs::read_dir(root).map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
            let kind = entry.file_type().map_err(|error| error.to_string())?;
            if kind.is_symlink() {
                return Err("symlinked brand source is unqualified".into());
            }
            if kind.is_dir() {
                found.extend(Source(&entry.path()).leaves()?);
            }
            if kind.is_file() {
                found.push(entry.path());
            }
            if found.len() > 4096 {
                return Err("brand source inventory exceeded its bound".into());
            }
        }
        found.sort();
        Ok(found)
    }

    pub(super) fn inputs(&self, apps: &[App]) -> Result<String, String> {
        let root = self.0;
        let snapshot = plumb::snapshot::Snapshot::read(root).map_err(|error| error.to_string())?;
        let mut entries = Vec::new();
        for entry in snapshot.entries() {
            if bears(entry.path(), apps) {
                entries.push((entry.path(), super::material::digest(entry.bytes())));
            }
        }
        for path in ["package.json", "pnpm-lock.yaml"] {
            tracked(root, path, &entries)?;
        }
        match root.join("pnpm-workspace.yaml").symlink_metadata() {
            Ok(_) => tracked(root, "pnpm-workspace.yaml", &entries)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => return Err(error.to_string()),
        }
        for app in apps {
            for name in ["package.json", "index.html", &app.config] {
                let path = app
                    .path
                    .join(name)
                    .to_str()
                    .ok_or("non-text app input path")?
                    .replace('\\', "/");
                tracked(root, &path, &entries)?;
            }
            Source(&root.join(&app.path)).tracked(root, &entries)?;
        }
        let bytes = serde_json::to_vec(&entries).map_err(|error| error.to_string())?;
        Ok(super::material::digest(&bytes))
    }
    fn tracked(&self, repository: &Path, entries: &[(&str, String)]) -> Result<(), String> {
        for directory in [self.0.join("src"), self.0.join("public")] {
            if !directory.exists() {
                continue;
            }
            for file in Source(&directory).leaves()? {
                let path = file
                    .strip_prefix(repository)
                    .map_err(|error| error.to_string())?
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                if !entries.iter().any(|(name, _)| *name == path) {
                    return Err("brand source is outside the tracked input world".into());
                }
            }
        }
        Ok(())
    }
}

fn tracked(root: &Path, path: &str, entries: &[(&str, String)]) -> Result<(), String> {
    if !entries.iter().any(|(name, _)| *name == path) {
        return Err(format!(
            "required brand input {path} is outside the tracked input world"
        ));
    }
    super::material::regular(root, path)?;
    Ok(())
}

pub(super) fn relative(path: &str) -> Result<(), String> {
    if path.is_empty() || path.contains(['\\', ':', '%', '?', '#']) || path.starts_with('/') {
        return Err("brand material path must be repository-relative".into());
    }
    if path
        .split('/')
        .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err("brand material path has an unsupported segment".into());
    }
    Ok(())
}

fn bears(path: &str, apps: &[App]) -> bool {
    if [
        ".npmrc",
        "package.json",
        "pnpm-lock.yaml",
        "pnpm-workspace.yaml",
    ]
    .contains(&path)
        || path.starts_with("packages/")
    {
        return true;
    }
    apps.iter().any(|app| {
        let prefix = app
            .path
            .components()
            .map(|part| part.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        if !prefix.is_empty() {
            return path.starts_with(&format!("{prefix}/"));
        }
        path == "index.html"
            || ["src/", "public/", "vite.config.", "tsconfig"]
                .iter()
                .any(|prefix| path.starts_with(prefix))
    })
}
