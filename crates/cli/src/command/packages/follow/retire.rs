use super::{Provider, Seat, State, work};
use std::path::Path;

pub(super) fn run(root: &Path, provider: &Provider<'_>) -> Result<String, String> {
    let seat = Seat::open(provider.repository).map_err(refused)?;
    let mut state = seat.read(provider.repository)?;
    retire(root, provider, &seat, &mut state)
}

fn refused(reason: impl std::fmt::Display) -> String {
    format!("retire refused: {reason}")
}

pub(super) fn retire(
    root: &Path,
    provider: &Provider<'_>,
    seat: &Seat,
    state: &mut State,
) -> Result<String, String> {
    let number = state.issue;
    if number == 0 {
        return Err(refused("no Auto is recorded for this repository"));
    }
    provider.writer(number).map_err(refused)?;
    if provider.issue(number)?["state"] != "closed" {
        return Err(refused(format!(
            "Auto issue #{number} is open; close the issue and pull by hand first"
        )));
    }
    let pull = closed(provider, state)?;
    if state.merged.is_some() {
        return Err(refused(format!(
            "Auto #{number} records a merge; rerun plumb follow to settle its closure"
        )));
    }
    if provider.absorption(number)?.is_some() {
        return Err(refused(format!(
            "Auto #{number} has a recorded absorption; rerun plumb follow to settle it"
        )));
    }
    if provider.completion(number)?.is_some() {
        return Err(refused(format!(
            "Auto #{number} has a recorded no-op receipt; rerun plumb follow to settle it"
        )));
    }
    let intent = provider.recorded(number)?;
    let known = |head: &str| {
        [&state.pushed, &state.candidate]
            .into_iter()
            .flatten()
            .any(|held| held == head)
            || intent.as_ref().is_some_and(|held| held.head == head)
            || state.plan.as_ref().is_some_and(|plan| plan.source == head)
    };
    let branch = format!("refs/heads/auto/{number}");
    let remote = provider.remote(number)?;
    if let Some(head) = &remote
        && !known(head)
    {
        return Err(refused(format!(
            "remote auto/{number} at {head} moved outside the recorded Auto; human judgment required"
        )));
    }
    let local = work::git(root, &["for-each-ref", "--format=%(objectname)", &branch])?;
    if !local.is_empty() && !known(&local) {
        return Err(refused(format!(
            "local auto/{number} at {local} moved outside the recorded Auto; human judgment required"
        )));
    }
    let tree = owned(root, seat, state)?;
    if let Some(head) = &remote {
        work::git(
            root,
            &[
                "push",
                &format!("--force-with-lease={branch}:{head}"),
                "origin",
                &format!(":{branch}"),
            ],
        )?;
    }
    if provider.remote(number)?.is_some() {
        return Err(format!(
            "Auto branch auto/{number} retirement was not read back"
        ));
    }
    if let Some(tree) = &tree {
        work::git(root, &["worktree", "remove", tree])?;
    }
    if !local.is_empty() {
        work::git(root, &["update-ref", "-d", &branch, &local])?;
    }
    *state = State {
        schema: "plumb.auto-state/v1".into(),
        repository: provider.repository.into(),
        ..State::default()
    };
    seat.write(state)?;
    Ok(format!(
        "Auto #{number} retired: issue closed and {pull}; remote auto/{number} {}; local branch {}; worktree {}; seat cleared, rerun plumb follow to open a fresh Auto from current main",
        remote.map_or("absent".into(), |head| format!(
            "removed by lease at {head}"
        )),
        if local.is_empty() {
            "absent".into()
        } else {
            format!("removed at {local}")
        },
        tree.unwrap_or_else(|| "absent".into()),
    ))
}

fn closed(provider: &Provider<'_>, state: &State) -> Result<String, String> {
    let Some(pull) = provider.find(state.issue)? else {
        if state.pull != 0 {
            return Err(refused(format!(
                "recorded Auto pull #{} is not found; human judgment required",
                state.pull
            )));
        }
        return Ok("no pull".into());
    };
    let id = pull["number"].as_u64().ok_or("Auto pull has no number")?;
    if state.pull != 0 && state.pull != id {
        return Err(refused(format!(
            "Auto pull #{id} disagrees with recorded pull #{}",
            state.pull
        )));
    }
    if pull["merged_at"].is_string() || pull["merged"] == true {
        return Err(refused(format!(
            "Auto pull #{id} is merged; rerun plumb follow to settle its closure"
        )));
    }
    if pull["state"] != "closed" {
        return Err(refused(format!(
            "Auto pull #{id} is open; close the issue and pull by hand first"
        )));
    }
    Ok(format!("pull #{id} closed unmerged"))
}

fn owned(root: &Path, seat: &Seat, state: &State) -> Result<Option<String>, String> {
    if state.worktree.as_os_str().is_empty() || !state.worktree.exists() {
        return Ok(None);
    }
    if state.worktree != seat.worktree()?.join("worktree") {
        return Err(refused(
            "Auto worktree path is outside its owned state seat",
        ));
    }
    work::Work {
        source: root,
        state,
    }
    .open()
    .map_err(refused)?;
    if !work::git(
        &state.worktree,
        &["status", "--porcelain", "--untracked-files=all"],
    )?
    .is_empty()
    {
        return Err(refused("Auto worktree holds uncommitted local payload"));
    }
    state
        .worktree
        .to_str()
        .map(|path| Some(path.to_string()))
        .ok_or_else(|| "Auto worktree path is not UTF-8".into())
}

pub(super) struct Leftover<'a> {
    pub root: &'a Path,
    pub provider: &'a Provider<'a>,
    pub state: &'a State,
}

impl Leftover<'_> {
    pub fn check(&self) -> Result<(), String> {
        for (number, tip) in work::Origin(self.root).heads()? {
            self.waived(number, &tip).map_err(|reason| {
                format!("leftover remote auto/{number} at {tip} blocks follow: {reason}")
            })?;
        }
        Ok(())
    }

    fn waived(&self, number: u64, tip: &str) -> Result<(), String> {
        let issue = self.provider.issue(number)?;
        if issue.get("pull_request").is_some() || issue["state"] != "closed" {
            return Err(format!("Auto issue #{number} is not a closed issue"));
        }
        self.pulled(number)?;
        self.idle(number)?;
        let origin = work::Origin(self.root);
        work::git(
            self.root,
            &["fetch", "origin", &format!("refs/heads/auto/{number}")],
        )?;
        if work::git(self.root, &["rev-parse", "FETCH_HEAD"])? != tip {
            return Err("the listed tip moved or is unreadable".into());
        }
        if !origin.merges(&format!("origin/main..{tip}"))?.is_empty() {
            return Err("origin/main.. range holds a merge commit".into());
        }
        if !origin.equivalent("origin/main", tip)? {
            return Err("git cherry against origin/main has a commit not marked -".into());
        }
        Ok(())
    }

    fn pulled(&self, number: u64) -> Result<(), String> {
        let branch = format!("auto/{number}");
        let repository = self.provider.repository;
        let owner = repository
            .split('/')
            .next()
            .ok_or("repository has no owner")?;
        let pulls = self.provider.pages(&format!(
            "repos/{repository}/pulls?state=all&head={owner}:{branch}&per_page=100"
        ))?;
        if pulls
            .iter()
            .any(|pull| pull["head"]["ref"].as_str() != Some(branch.as_str()))
        {
            return Err("a pull request for this head names another branch".into());
        }
        if pulls.iter().any(|pull| pull["state"] != "closed") {
            return Err("an open pull request still has this head".into());
        }
        let merged = pulls
            .iter()
            .filter(|pull| pull["merged_at"].is_string())
            .count();
        if merged != 1 {
            return Err(format!(
                "exactly one merged pull request must have this head; found {merged}"
            ));
        }
        Ok(())
    }

    fn idle(&self, number: u64) -> Result<(), String> {
        if self.state.issue == number {
            return Err("the running follow still records this Auto".into());
        }
        let held = format!("branch refs/heads/auto/{number}");
        if work::git(self.root, &["worktree", "list", "--porcelain"])?
            .lines()
            .any(|line| line == held)
        {
            return Err("a local worktree still holds this Auto branch".into());
        }
        Ok(())
    }
}
