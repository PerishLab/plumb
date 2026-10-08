use super::preview::App;
use plumb::snapshot::{Refusal, Snapshot};
use std::path::Path;

pub(crate) fn read(
    root: &Path,
    apps: &Result<Vec<App>, String>,
    snapshot: Result<&Snapshot, &Refusal>,
) -> Result<(), String> {
    let Ok(apps) = apps else {
        return Ok(());
    };
    if apps.is_empty() {
        return Ok(());
    }
    let source = Source {
        root,
        snapshot: snapshot.map_err(|error| error.to_string())?,
    };
    for app in apps {
        source
            .judge(app)
            .map_err(|error| format!("preview app {}: {error}", app.path))?;
    }
    Ok(())
}

struct Source<'a> {
    root: &'a Path,
    snapshot: &'a Snapshot,
}

impl Source<'_> {
    fn document(&self, path: &str, comments: bool) -> Result<serde_json::Value, String> {
        super::preview::regular(self.root, self.snapshot, path)?;
        let entry = self
            .snapshot
            .entries()
            .iter()
            .find(|entry| entry.path() == path)
            .ok_or_else(|| format!("missing tracked {path}"))?;
        super::document::read(entry.bytes(), comments).map_err(|error| format!("{path}: {error}"))
    }

    fn judge(&self, app: &App) -> Result<(), String> {
        let package = format!("{}/package.json", app.path);
        super::package::judge(&self.document(&package, false)?, &app.package)?;
        let path = format!("{}/wrangler.jsonc", app.path);
        let worker = super::worker::Worker::read(self.document(&path, true)?)?;
        self.output(&format!("{}/{}", app.path, worker.assets.path()?))?;
        self.aliases(app, &worker)
    }

    fn aliases(&self, app: &App, worker: &super::worker::Worker) -> Result<(), String> {
        for entry in self.snapshot.entries() {
            let path = entry.path();
            let name = path.rsplit('/').next().unwrap_or(path);
            match name {
                "package.json" if path != format!("{}/package.json", app.path) => {
                    self.package(path, app)?;
                }
                "wrangler.jsonc" if path != format!("{}/wrangler.jsonc", app.path) => {
                    self.worker(path, worker)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn package(&self, path: &str, app: &App) -> Result<(), String> {
        let other = self.document(path, false)?;
        if !other.is_object() || other["name"] == app.package {
            return Err(format!("{path} is ambiguous or aliases the static package"));
        }
        Ok(())
    }

    fn worker(&self, path: &str, worker: &super::worker::Worker) -> Result<(), String> {
        let other = self.document(path, true)?;
        let alias = other["account_id"] == worker.account && other["name"] == worker.name;
        if !other.is_object() || alias {
            return Err(format!("{path} is ambiguous or aliases the static Worker"));
        }
        Ok(())
    }

    fn output(&self, path: &str) -> Result<(), String> {
        let mut held = self.root.to_path_buf();
        for part in path.split('/') {
            held.push(part);
            match std::fs::symlink_metadata(&held) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err("static output traverses a symlink".into());
                }
                Ok(metadata) if !metadata.is_dir() => {
                    return Err("static output ancestry must contain directories".into());
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                Err(error) => return Err(error.to_string()),
            }
        }
        Ok(())
    }
}
