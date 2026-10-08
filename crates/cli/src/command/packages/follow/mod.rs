mod closure;
mod engine;
mod issue;
mod provider;
mod pull;
mod refresh;
mod scope;
mod state;
#[cfg(all(test, unix))]
#[path = "../tests.rs"]
mod tests;
mod work;

use provider::Provider;
use serde_json::json;
use state::{Seat, State};
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
    match resume(input, &provider, &seat, &mut state) {
        Ok(message) => Ok(message),
        Err(error) => {
            if state.issue == 0 {
                return Err(format!(
                    "{error}; no Auto issue or pull could be recorded; reread the provider before retry"
                ));
            }
            let body = format!(
                "Auto follow stopped: {error}\n\nExisting pull: {}. Registered paths and required Guard remain unchanged. Resolve the blocker, then rerun plumb follow.",
                if state.pull == 0 {
                    "none could be recorded; reread the provider before resuming".into()
                } else {
                    format!("#{}", state.pull)
                }
            );
            if let Err(recording) = provider.comment(state.issue, &body) {
                return Err(format!(
                    "{error}; failure comment could not be recorded: {recording}"
                ));
            }
            Err(error)
        }
    }
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
    if state.issue == 0 {
        state.issue = issue::find(provider)?.unwrap_or(0);
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
        if !known && provider.find(state.issue)?.is_none() {
            return Err("unrecorded Auto remote branch has no registered pull relationship".into());
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
