use super::{Provider, Seat, State, issue, scope, work};
use std::path::Path;

pub(super) fn close(
    root: &Path,
    provider: &Provider<'_>,
    seat: &Seat,
    state: &mut State,
) -> Result<String, String> {
    let merge = state.merged.as_ref().ok_or("Auto closure has no merge")?;
    let guard = state
        .guard
        .as_ref()
        .ok_or("Auto closure has no Guard run")?;
    let candidate = state
        .candidate
        .as_ref()
        .ok_or("Auto merge has no exact candidate identity")?;
    work::git(root, &["fetch", "origin"])?;
    work::git(
        root,
        &["fetch", "origin", &format!("refs/pull/{}/head", state.pull)],
    )?;
    if work::git(root, &["rev-parse", "FETCH_HEAD"])? != *candidate {
        return Err("Auto merged pull head differs during acquisition".into());
    }
    let base = work::git(root, &["rev-parse", &format!("{candidate}^1")])?;
    scope::check(root, &base, candidate)?;
    work::git(root, &["merge-base", "--is-ancestor", merge, "origin/main"])?;
    issue::snapshot(provider, state.issue)?;
    let landed =
        plumb::delivery::landed(root, candidate, merge).map_err(|error| error.to_string())?;
    let body = format!(
        "Auto follow acceptance satisfied. Merge commit: https://github.com/{}/commit/{merge}. Required organization Guard: {guard}. Exact candidate {} was read back with the same parent, tree and Guard proof.\n\n<!-- plumb.auto-closure/v1 -->",
        provider.repository, candidate
    );
    provider.comment(state.issue, &body)?;
    let result = format!(
        "Auto #{} completed via pull #{} at {merge}",
        state.issue, state.pull
    );
    if state.worktree.exists() {
        work::Work {
            source: root,
            state,
        }
        .open()?;
        let local = work::git(&state.worktree, &["rev-parse", "HEAD"])?;
        work::git(
            root,
            &[
                "worktree",
                "remove",
                state
                    .worktree
                    .to_str()
                    .ok_or("worktree path is not UTF-8")?,
            ],
        )?;
        if state.plan.as_ref().is_some_and(|plan| plan.source == local) {
            work::git(
                root,
                &[
                    "update-ref",
                    &format!("refs/heads/auto/{}", state.issue),
                    candidate,
                    &local,
                ],
            )?;
        }
    }
    let retired = landed.retire(
        root,
        "origin",
        &[plumb::delivery::Pushed {
            branch: format!("auto/{}", state.issue),
            head: candidate.clone(),
        }],
    );
    if !retired.kept.is_empty() {
        provider.comment(
            state.issue,
            &format!(
                "Auto cleanup retained branches with explicit reasons: {}",
                serde_json::to_string(&retired.kept).map_err(|error| error.to_string())?
            ),
        )?;
    }
    let current = provider.issue(state.issue)?;
    let accepted = issue::text(&current, "body")?.replace("- [ ]", "- [x]");
    provider.patch(
        &format!("repos/{}/issues/{}", provider.repository, state.issue),
        &serde_json::json!({"body": accepted, "state": "closed"}),
    )?;
    if provider.issue(state.issue)?["state"] != "closed" {
        return Err("Auto issue closure was not read back".into());
    }
    *state = State {
        schema: "plumb.auto-state/v1".into(),
        repository: provider.repository.into(),
        ..State::default()
    };
    seat.write(state)?;
    Ok(result)
}
pub(super) fn idle(
    root: &Path,
    provider: &Provider<'_>,
    seat: &Seat,
    state: &mut State,
) -> Result<String, String> {
    if state.pull != 0 {
        return super::absorb::Absorption {
            root,
            provider,
            seat,
        }
        .absorb(state);
    }
    let main = work::git(root, &["rev-parse", "origin/main"])?;
    let resolution = plumb::packages::locked(&state.worktree, "follow")?;
    let snapshot = issue::snapshot(provider, state.issue)?;
    if snapshot.state != "OPEN" || state.plan.is_some() {
        return Err(
            "qualified Auto no-op requires a failed preflight with no associated pull or delivery"
                .into(),
        );
    }
    if state.merged.is_some() || state.guard.is_some() || provider.find(state.issue)?.is_some() {
        return Err("qualified Auto no-op has associated delivery progress".into());
    }
    let intent = provider.recorded(state.issue)?;
    let remote = provider.remote(state.issue)?;
    let retired = match intent {
        Some(held) => {
            if held.head != held.base || held.previous.is_some() {
                return Err("qualified Auto no-op refuses nonempty publication history".into());
            }
            if remote.as_ref().is_some_and(|head| head != &held.head)
                || state.pushed.as_ref().is_some_and(|head| head != &held.head)
            {
                return Err(
                    "qualified Auto no-op refuses nonempty or moved publication progress".into(),
                );
            }
            if work::git(root, &["rev-parse", &format!("{}^{{tree}}", held.head)])? != held.tree {
                return Err("owned empty publication intent tree disagrees".into());
            }
            Some(held.head)
        }
        None if remote.is_none() && state.pushed.is_none() && state.candidate.is_none() => None,
        None => return Err("qualified Auto no-op has unrecorded publication progress".into()),
    };
    let latest = plumb::packages::Plan::read(&state.worktree, "follow")?;
    latest.verify(&state.worktree)?;
    work::current(root, &main)?;
    if resolution.tree != work::git(root, &["rev-parse", &format!("{main}^{{tree}}")])? {
        return Err("qualified Auto no-op resolution differs from current main".into());
    }
    if let Some(head) = remote {
        let branch = format!("refs/heads/auto/{}", state.issue);
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
    if provider.remote(state.issue)?.is_some() {
        return Err("qualified Auto no-op empty-ref retirement was not read back".into());
    }
    let receipt = issue::Noop {
        schema: "plumb.auto-noop/v1".into(),
        repository: provider.repository.into(),
        issue: state.issue,
        main: main.clone(),
        resolution,
        retired,
    };
    let body = format!(
        "Auto follow qualified no-op. Current main: https://github.com/{}/commit/{main}. Latest-stable resolution found no registered delta; no associated pull exists and the owned empty ref was read back absent. No dependency change, pull, merge or Guard was produced.\n\n<!-- plumb.auto-noop/v1\n{}\n-->",
        provider.repository,
        serde_json::to_string(&receipt).map_err(|error| error.to_string())?
    );
    provider.comment(state.issue, &body)?;
    work::current(root, &main)?;
    provider.published(&snapshot)?;
    work::Work {
        source: root,
        state,
    }
    .retire(&receipt.resolution.tree)?;
    if provider.find(state.issue)?.is_some() || provider.remote(state.issue)?.is_some() {
        return Err(
            "Auto no-op provider progress moved before closure; reread before retry".into(),
        );
    }
    provider.patch(&format!("repos/{}/issues/{}", provider.repository, state.issue),
        &serde_json::json!({"body": issue::body(provider.repository).replace("- [ ]", "- [x]"), "state":"closed"}))?;
    if !resume(root, provider, seat, state)? {
        return Err("qualified Auto no-op closure was not read back".into());
    }
    Ok(format!(
        "Auto #{} completed as a qualified no-op at {main}; no pull, merge or Guard produced",
        receipt.issue
    ))
}

pub(super) fn resume(
    root: &Path,
    provider: &Provider<'_>,
    seat: &Seat,
    state: &mut State,
) -> Result<bool, String> {
    let issue = provider.issue(state.issue)?;
    if issue["state"] != "closed" {
        return Ok(false);
    }
    if provider.absorption(state.issue)?.is_some() {
        super::absorb::Absorption {
            root,
            provider,
            seat,
        }
        .settle(state)?;
        return Ok(true);
    }
    let found = provider.completion(state.issue)?;
    let Some(receipt) = found else {
        return Ok(false);
    };
    if (
        receipt.schema.as_str(),
        receipt.repository.as_str(),
        receipt.issue,
    ) != ("plumb.auto-noop/v1", provider.repository, state.issue)
        || receipt.resolution.context != "follow"
        || issue["body"] != issue::body(provider.repository).replace("- [ ]", "- [x]")
    {
        return Err("closed Auto no-op identity or acceptance disagrees".into());
    }
    if !matches!(receipt.main.len(), 40 | 64)
        || !receipt.main.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("closed Auto no-op has an invalid main identity".into());
    }
    issue::snapshot(provider, state.issue)?;
    if let Some(head) = &receipt.retired {
        let intent = provider
            .recorded(state.issue)?
            .ok_or("closed Auto no-op lost its publication intent")?;
        if intent.head != *head || intent.base != *head || intent.previous.is_some() {
            return Err("closed Auto no-op retirement differs from its owned empty intent".into());
        }
        if work::git(root, &["rev-parse", &format!("{head}^{{tree}}")])? != intent.tree {
            return Err("closed Auto no-op owned intent tree disagrees".into());
        }
    }
    if provider.find(state.issue)?.is_some()
        || provider.remote(state.issue)?.is_some()
        || work::git(root, &["rev-parse", &format!("{}^{{tree}}", receipt.main)])?
            != receipt.resolution.tree
    {
        return Err("closed Auto no-op no longer agrees with provider and Git evidence".into());
    }
    work::git(
        root,
        &["merge-base", "--is-ancestor", &receipt.main, "origin/main"],
    )?;
    work::Work {
        source: root,
        state,
    }
    .retire(&receipt.resolution.tree)?;
    *state = State {
        schema: "plumb.auto-state/v1".into(),
        repository: provider.repository.into(),
        ..State::default()
    };
    seat.write(state)?;
    Ok(true)
}
