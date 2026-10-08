use super::{Provider, Seat, State, issue, work};
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
    work::git(root, &["fetch", "origin", candidate])?;
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
