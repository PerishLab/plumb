use crate::shape::web::{Activation, Setup};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct App {
    pub(super) path: PathBuf,
    pub(super) setup: Setup,
}

pub(super) struct Source<'a>(pub(super) &'a Path);

impl Source<'_> {
    pub(super) fn apps(&self) -> Result<Vec<App>, String> {
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
            if let Some(app) = self.app(&path)? {
                found.push(app)
            }
        }
        Ok(found)
    }

    fn app(&self, path: &Path) -> Result<Option<App>, String> {
        let root = self.0;
        let manifest = root.join(path).join("package.json");
        if !manifest.is_file() {
            return Ok(None);
        }
        let bytes = std::fs::read(manifest).map_err(|error| error.to_string())?;
        let manifest: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        if !has(&manifest, "vite") || !has(&manifest, "svelte") {
            return Ok(None);
        }
        let configs = [
            "vite.config.ts",
            "vite.config.mts",
            "vite.config.js",
            "vite.config.mjs",
        ]
        .into_iter()
        .map(|name| root.join(path).join(name))
        .filter(|path| path.exists())
        .collect::<Vec<_>>();
        if configs.len() != 1 {
            return Err(format!(
                "{} needs one readable static Vite configuration",
                path.display()
            ));
        }
        let text = std::fs::read_to_string(&configs[0]).map_err(|error| error.to_string())?;
        let setup = match Activation::read(&text) {
            Activation::Active(setup) => setup,
            Activation::Missing => {
                return Err(format!(
                    "{} has no imported Design plugin activation",
                    path.display()
                ));
            }
            Activation::Unknown(reason) => {
                return Err(format!(
                    "{} Vite configuration is unqualified: {reason}",
                    path.display()
                ));
            }
        };
        if let Some(problem) = &setup.problem {
            return Err(format!(
                "{} brand configuration is unqualified: {problem}",
                path.display()
            ));
        }
        if setup.options.get("brand").is_none() {
            return Err(format!(
                "{} has no explicit brand product/origin; adoption is unqualified",
                path.display()
            ));
        }
        relative(&setup.output)?;
        if !setup.base.starts_with('/')
            || setup.base.starts_with("//")
            || !setup.base.ends_with('/')
        {
            return Err("brand output requires a local absolute Vite base ending in /".into());
        }
        Ok(Some(App {
            path: path.into(),
            setup,
        }))
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
        for app in apps {
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

fn has(manifest: &serde_json::Value, name: &str) -> bool {
    ["dependencies", "devDependencies", "peerDependencies"]
        .iter()
        .any(|section| manifest[section][name].is_string())
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
