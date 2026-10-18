mod absorb;
mod closure;
mod engine;
mod issue;
mod provider;
mod pull;
mod refresh;
mod scope;
#[cfg(all(test, unix))]
#[path = "../tests.rs"]
mod tests;
mod work;

use provider::Provider;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub(crate) struct Input {
    pub root: PathBuf,
    pub github: PathBuf,
    pub json: bool,
}

pub(crate) fn run(input: Input) -> i32 {
    match execute(&input) {
        Ok(message) => report(&input, &message),
        Err(error) => {
            eprintln!("plumb follow: {error}");
            1
        }
    }
}

fn report(input: &Input, message: &str) -> i32 {
    if input.json {
        println!(
            "{}",
            json!({"schema":"plumb.follow/v1", "ok":true,"message":message})
        );
    } else {
        println!("{message}");
    }
    0
}

fn execute(input: &Input) -> Result<String, String> {
    let repository = repository(&input.root)?;
    let provider = Provider {
        command: &input.github,
        repository: &repository,
    };
    let seat = Seat::open(&repository)?;
    let mut state = seat.read(&repository)?;
    resume(input, &provider, &seat, &mut state).map_err(|error| provider.stopped(&state, error))
}

fn resume(
    input: &Input,
    provider: &Provider<'_>,
    seat: &Seat,
    state: &mut State,
) -> Result<String, String> {
    if !work::git(
        &input.root,
        &["status", "--porcelain", "--untracked-files=all"],
    )?
    .is_empty()
    {
        return Err("follow requires a clean source checkout".into());
    }
    work::git(&input.root, &["fetch", "origin"])?;
    if state.issue != 0 {
        provider.writer(state.issue)?;
        closure::resume(&input.root, provider, seat, state)?;
    }
    if state.issue == 0 {
        state.issue = issue::find(provider)?.unwrap_or(0);
        if state.issue != 0 {
            provider.writer(state.issue)?;
        }
    }
    if state.issue == 0 {
        let tree = work::git(&input.root, &["rev-parse", "origin/main^{tree}"])?;
        let probe = crate::command::guard::precommit::tree::Index::new(&input.root, &tree)?;
        if let Err(error) = super::resolve(&probe.root, "follow") {
            state.issue = issue::create(provider).map_err(|recording| {
                format!("{error}; no Auto issue or pull could be created: {recording}")
            })?;
            seat.write(state)?;
            provider.comment(state.issue, &format!("Auto follow stopped before a pull request was created: {error}. Resolve the blocker, then rerun plumb follow."))?;
            return Err(error);
        }
        if work::git(&probe.root, &["diff", "--name-only", "HEAD"])?.is_empty() {
            return Ok("already current; no Auto issue or pull request created".into());
        }
        state.issue = issue::create(provider)?;
    }
    issue::snapshot(provider, state.issue)?;
    let owned = seat.worktree()?.join("worktree");
    if state.worktree.as_os_str().is_empty() {
        state.worktree = owned;
    } else if state.worktree != owned {
        return Err("Auto worktree path is outside its owned state seat".into());
    }
    if let Some(head) = provider.remote(state.issue)? {
        let known = state.pushed.as_ref() == Some(&head) || state.candidate.as_ref() == Some(&head);
        if !known && (state.pushed.is_some() || state.candidate.is_some()) {
            return Err("Auto remote head moved outside recorded recovery state".into());
        }
        let recorded = provider.recorded(state.issue)?;
        if let Some(intent) = &recorded
            && intent.head != head
            && intent.previous.as_ref() != Some(&head)
        {
            return Err("Auto remote head moved outside recorded publication intent".into());
        }
        if !known
            && provider.find(state.issue)?.is_none()
            && recorded.as_ref().is_none_or(|intent| intent.head != head)
        {
            return Err("unrecorded Auto remote branch has no registered pull relationship".into());
        }
        work::git(
            &input.root,
            &[
                "fetch",
                "origin",
                &format!("refs/heads/auto/{}", state.issue),
            ],
        )?;
        if work::git(&input.root, &["rev-parse", "FETCH_HEAD"])? != head {
            return Err("Auto remote head moved during acquisition".into());
        }
        if let Some(intent) = recorded.filter(|intent| intent.head == head) {
            if work::git(&input.root, &["rev-parse", &format!("{head}^{{tree}}")])? != intent.tree {
                return Err("Auto acquired tree differs from publication intent".into());
            }
            scope::check(&input.root, &intent.base, &head)?;
        } else {
            let base = work::git(&input.root, &["merge-base", "origin/main", &head])?;
            scope::check(&input.root, &base, &head)?;
        }
        state.pushed = Some(head);
    }
    seat.write(state)?;
    engine::advance(&input.root, provider, seat, state)
}

fn repository(root: &Path) -> Result<String, String> {
    let remote = work::git(root, &["remote", "get-url", "origin"])?;
    let path = remote
        .strip_prefix("git@github.com:")
        .or_else(|| remote.strip_prefix("https://github.com/"))
        .or_else(|| remote.strip_prefix("ssh://git@github.com/"))
        .ok_or("follow requires a GitHub origin")?
        .trim_end_matches(".git");
    if path.split('/').count() != 2 || path.split('/').any(|part| part.is_empty()) {
        return Err("GitHub origin has no exact owner/repository".into());
    }
    Ok(path.into())
}

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
