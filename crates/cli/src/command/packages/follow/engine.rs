use super::{Provider, Seat, State, issue, pull, refresh, work};
use std::path::Path;

pub(super) fn advance(
    root: &Path,
    provider: &Provider<'_>,
    seat: &Seat,
    state: &mut State,
) -> Result<String, String> {
    if let Some(pull) = provider.find(state.issue)? {
        state.pull = pull["number"].as_u64().ok_or("Auto pull has no number")?;
        if pull["merged_at"].is_string() {
            let merge = issue::text(&pull, "merge_commit_sha")?;
            let head = issue::text(&pull["head"], "sha")?;
            if state
                .plan
                .as_ref()
                .is_some_and(|plan| plan.candidate != head)
            {
                return Err("merged Auto head differs from recorded candidate".into());
            }
            state.candidate = Some(head.clone());
            state.merged = Some(merge);
            state.guard = Some(provider.guard(&head)?);
            seat.write(state)?;
            return super::closure::close(root, provider, seat, state);
        }
        if pull["state"] != "open" {
            return Err("Auto pull was closed without merge; human judgment required".into());
        }
    }
    let snapshot = issue::snapshot(provider, state.issue)?;
    if snapshot.state != "OPEN" {
        return Err("Auto issue is closed without a verified merged pull".into());
    }
    let work = work::Work {
        source: root,
        state,
    };
    work.open()?;
    if let Some(commit) = refresh::prepare(state)? {
        state.candidate = Some(commit.clone());
        state.plan = None;
        seat.write(state)?;
        refresh::apply(&state.worktree, &commit)?;
    }
    let work = work::Work {
        source: root,
        state,
    };
    crate::command::packages::resolve(&state.worktree, "follow")?;
    let head = work.commit()?;
    let base = work::git(&state.worktree, &["rev-parse", "origin/main"])?;
    work::boundary(&state.worktree, &base, &head)?;
    state.candidate = Some(head.clone());
    seat.write(state)?;
    let published = provider.publish(
        state.issue,
        pull::Publication {
            root: &state.worktree,
            head: &head,
            previous: state.pushed.as_deref(),
        },
    )?;
    state.pushed = Some(head);
    state.pull = published["number"]
        .as_u64()
        .ok_or("Auto pull has no number")?;
    seat.write(state)?;
    crate::command::guard::precommit::branch::renew(&state.worktree)?;
    let current = issue::snapshot(provider, state.issue)?;
    let narrative = plumb::delivery::Narrative {
        title: "Follow the latest first-party packages".into(),
        body: format!(
            "Refs #{}.\n\nRegistered follow operation; manifests and lockfiles only. Exact local Guard proof and required organization Guard precede merge. Read-back merge and Guard run settle Auto closure.",
            state.issue
        ),
    };
    let observed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_secs();
    let request = plumb::delivery::Request {
        root: &state.worktree,
        repository: provider.repository,
        issue: &current,
        observed,
        base: "main",
        pull: &narrative,
    };
    let authority = plumb::guard::Authority::released()?;
    let plan = plumb::delivery::prepare(request, &authority).map_err(|error| error.to_string())?;
    work::boundary(&state.worktree, &plan.target, &plan.candidate)?;
    state.candidate = Some(plan.candidate.clone());
    state.plan = Some(plan.clone());
    seat.write(state)?;
    provider.publish(
        state.issue,
        pull::Publication {
            root: &state.worktree,
            head: &plan.candidate,
            previous: state.pushed.as_deref(),
        },
    )?;
    state.pushed = Some(plan.candidate.clone());
    seat.write(state)?;
    let guard = wait(provider, &plan.candidate)?;
    let latest = plumb::packages::Plan::read(&state.worktree, "follow")?;
    latest.verify(&state.worktree)?;
    let snapshot = issue::snapshot(provider, state.issue)?;
    let request = plumb::delivery::Request {
        root: &state.worktree,
        repository: provider.repository,
        issue: &snapshot,
        observed,
        base: "main",
        pull: &narrative,
    };
    work::git(&state.worktree, &["fetch", "origin"])?;
    plumb::delivery::revalidate(request, &plan, &authority).map_err(|error| error.to_string())?;
    let squash = plumb::delivery::Squash::read(&state.worktree, &plan.candidate)
        .map_err(|error| error.to_string())?;
    provider.command(&squash.arguments(provider.repository, state.pull))?;
    let merged = provider.api(
        &format!("repos/{}/pulls/{}", provider.repository, state.pull),
        None,
    )?;
    if merged["merged"] != true {
        return Err("provider did not confirm Auto merge".into());
    }
    let merge = issue::text(&merged, "merge_commit_sha")?;
    work::git(&state.worktree, &["fetch", "origin"])?;
    plumb::delivery::landed(&state.worktree, &plan.candidate, &merge)
        .map_err(|error| error.to_string())?;
    state.merged = Some(merge);
    state.guard = Some(guard);
    seat.write(state)?;
    super::closure::close(root, provider, seat, state)
}

fn wait(provider: &Provider<'_>, head: &str) -> Result<String, String> {
    for _ in 0..90 {
        match provider.guard(head) {
            Ok(run) => return Ok(run),
            Err(error) if error.contains("pending") => {
                std::thread::sleep(std::time::Duration::from_secs(10))
            }
            Err(error) => return Err(error),
        }
    }
    Err("organization Guard did not settle within the bounded wait".into())
}
