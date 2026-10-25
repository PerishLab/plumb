use super::provider::Patience;
use super::{Provider, Seat, State, issue, pull, refresh, scope, work};
use std::path::Path;
use std::time::Instant;

pub(super) fn advance(
    root: &Path,
    provider: &Provider<'_>,
    seat: &Seat,
    state: &mut State,
) -> Result<String, String> {
    let mut failed = None;
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
        if pull["state"] != "open" && provider.absorption(state.issue)?.is_some() {
            return super::absorb::Absorption {
                root,
                provider,
                seat,
            }
            .settle(state);
        }
        let head = issue::text(&pull["head"], "sha")?;
        if pull["state"] != "open" {
            return Err("Auto pull was closed without merge; human judgment required; if the close is final, run plumb follow --retire, then rerun plumb follow".into());
        }
        if let Err(error) = provider.guard(&head)
            && !error.starts_with("organization Guard is pending;")
            && current(root, &head)?
        {
            let work = work::Work {
                source: root,
                state,
            };
            if !error.starts_with("organization Guard failed:") || !work.changed(provider, &head)? {
                return Err(error);
            }
            failed = Some((head, error));
        }
    }
    let snapshot = issue::snapshot(provider, state.issue)?;
    if snapshot.state != "OPEN" {
        return Err("Auto issue is closed without a verified merged pull; human judgment required; if the close is final, run plumb follow --retire, then rerun plumb follow".into());
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
    let base = work::git(&state.worktree, &["rev-parse", "origin/main"])?;
    crate::command::packages::resolve(&state.worktree, "follow")?;
    if let Some((head, error)) = failed
        && work::git(&state.worktree, &["write-tree"])?
            == work::git(&state.worktree, &["rev-parse", &format!("{head}^{{tree}}")])?
    {
        return Err(error);
    }
    if work::git(&state.worktree, &["write-tree"])?
        == work::git(&state.worktree, &["rev-parse", &format!("{base}^{{tree}}")])?
    {
        return super::closure::idle(root, provider, seat, state);
    }
    let head = work.commit()?;
    scope::check(&state.worktree, &base, &head)?;
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
    let authority = plumb::guard::Authority::running()?;
    let mut plan =
        plumb::delivery::prepare(request, &authority).map_err(|error| error.to_string())?;
    scope::check(&state.worktree, &plan.target, &plan.candidate)?;
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
    plan.issue = provider.published(&plan.issue)?;
    state.plan = Some(plan.clone());
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
    merge(provider, state.pull, &squash)?;
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

fn current(root: &Path, head: &str) -> Result<bool, String> {
    let base = work::git(root, &["rev-parse", "origin/main"])?;
    Ok(work::git(root, &["merge-base", head, &base])? == base)
}

fn merge(
    provider: &Provider<'_>,
    pull: u64,
    squash: &plumb::delivery::Squash,
) -> Result<(), String> {
    let Patience { deadline, pause } = provider.patience;
    let deadline = Instant::now() + deadline;
    loop {
        ready(provider, pull, &squash.candidate, deadline)?;
        match provider.command(&squash.arguments(provider.repository, pull)) {
            Ok(_) => return Ok(()),
            Err(error) if refused(&error) && Instant::now() + pause < deadline => {
                std::thread::sleep(pause)
            }
            Err(error) => return Err(error),
        }
    }
}

fn ready(
    provider: &Provider<'_>,
    pull: u64,
    candidate: &str,
    deadline: Instant,
) -> Result<(), String> {
    let pause = provider.patience.pause;
    loop {
        let bytes = provider.command(
            &[
                "pr",
                "view",
                &pull.to_string(),
                "-R",
                provider.repository,
                "--json",
                "headRefOid,mergeStateStatus",
            ]
            .map(str::to_string),
        )?;
        let view: serde_json::Value = serde_json::from_slice(&bytes)
            .map_err(|error| format!("Auto pull readiness is unreadable: {error}"))?;
        let head = issue::text(&view, "headRefOid")?;
        let status = issue::text(&view, "mergeStateStatus")?;
        if head != candidate {
            return Err(format!(
                "Auto pull head {head} is not the exact candidate {candidate}; merge refused"
            ));
        }
        match status.as_str() {
            "CLEAN" => return Ok(()),
            "BLOCKED" | "UNKNOWN" if Instant::now() + pause < deadline => std::thread::sleep(pause),
            "BLOCKED" | "UNKNOWN" => {
                return Err(format!(
                    "Auto pull did not become mergeable within the bounded wait; last mergeStateStatus {status}"
                ));
            }
            _ => {
                return Err(format!(
                    "Auto pull is not mergeable: mergeStateStatus {status}; merge refused"
                ));
            }
        }
    }
}

fn refused(error: &str) -> bool {
    error.contains("base branch policy prohibits the merge") || error.contains("not mergeable")
}

fn wait(provider: &Provider<'_>, head: &str) -> Result<String, String> {
    for _ in 0..90 {
        match provider.guard(head) {
            Ok(run) => return Ok(run),
            Err(error) if error.starts_with("organization Guard is pending;") => {
                std::thread::sleep(std::time::Duration::from_secs(10))
            }
            Err(error) => return Err(error),
        }
    }
    Err("organization Guard did not settle within the bounded wait".into())
}
