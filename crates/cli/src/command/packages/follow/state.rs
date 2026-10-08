use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct State {
    pub schema: String,
    pub repository: String,
    pub issue: u64,
    pub worktree: PathBuf,
    pub plan: Option<plumb::delivery::Plan>,
    pub pull: u64,
    pub pushed: Option<String>,
    pub candidate: Option<String>,
    pub merged: Option<String>,
    pub guard: Option<String>,
}

pub(super) struct Seat {
    path: PathBuf,
    _lock: File,
}

impl Seat {
    #[cfg(all(test, unix))]
    pub fn fixture(root: &Path) -> Self {
        Self {
            path: root.join("state.json"),
            _lock: File::create(root.join("lease")).unwrap(),
        }
    }

    pub fn open(repository: &str) -> Result<Self, String> {
        let home = plumb::config::value("PLUMB_HOME")
            .map(PathBuf::from)
            .or_else(|| plumb::config::data("plumb"))
            .ok_or("follow has no PLUMB_HOME")?;
        let root = home
            .join("state/follow")
            .join(plumb::depot::sha(repository.as_bytes()));
        std::fs::create_dir_all(&root).map_err(|error| error.to_string())?;
        let root = root.canonicalize().map_err(|error| error.to_string())?;
        for path in [root.join("lease"), root.join("state.json")] {
            if let Ok(metadata) = std::fs::symlink_metadata(&path)
                && !metadata.is_file()
            {
                return Err(format!(
                    "Auto state requires a regular file: {}",
                    path.display()
                ));
            }
        }
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root.join("lease"))
            .map_err(|error| error.to_string())?;
        lock.try_lock()
            .map_err(|error| format!("follow already holds this repository: {error}"))?;
        Ok(Self {
            path: root.join("state.json"),
            _lock: lock,
        })
    }

    pub fn read(&self, repository: &str) -> Result<State, String> {
        if !self.path.exists() {
            return Ok(State {
                schema: "plumb.auto-state/v1".into(),
                repository: repository.into(),
                ..State::default()
            });
        }
        let bytes = std::fs::read(&self.path).map_err(|error| error.to_string())?;
        let state: State = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        if state.schema != "plumb.auto-state/v1" || state.repository != repository {
            return Err("follow state schema or repository disagrees".into());
        }
        for head in [&state.pushed, &state.candidate, &state.merged]
            .into_iter()
            .flatten()
        {
            if !matches!(head.len(), 40 | 64) || !head.bytes().all(|byte| byte.is_ascii_hexdigit())
            {
                return Err("Auto state has an invalid commit identity".into());
            }
        }
        Ok(state)
    }

    pub fn write(&self, state: &State) -> Result<(), String> {
        let parent = self.path.parent().ok_or("state has no parent")?;
        let mut file =
            tempfile::NamedTempFile::new_in(parent).map_err(|error| error.to_string())?;
        serde_json::to_writer(&mut file, state).map_err(|error| error.to_string())?;
        file.flush().map_err(|error| error.to_string())?;
        file.persist(&self.path)
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn worktree(&self) -> Result<&Path, String> {
        self.path
            .parent()
            .ok_or_else(|| "state has no parent".into())
    }
}
