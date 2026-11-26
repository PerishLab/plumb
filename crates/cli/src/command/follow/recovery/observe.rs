use super::super::{
    Provider, Seat, State, closure, issue,
    provider::{Intent, Source},
    scope, work,
};
use std::collections::BTreeSet;
use std::path::Path;

pub(super) struct Recovery<'a> {
    pub root: &'a Path,
    pub provider: &'a Provider<'a>,
    pub state: &'a State,
}

#[derive(Eq, PartialEq)]
pub(super) struct Evidence {
    issue: plumb::delivery::Snapshot,
    pub head: String,
    pub merge: String,
    pub guard: String,
    pub candidate: plumb::guard::Descriptor,
    pub history: Vec<Intent>,
    main: String,
}

impl Recovery<'_> {
    pub(super) fn observe(&self) -> Result<Evidence, String> {
        let provider = self.provider;
        let state = self.state;
        let number = state.issue;
        provider.writer(number)?;
        let issue = issue::snapshot(provider, number)?;
        let ticket = provider.issue(number)?;
        if issue.state != "CLOSED" || ticket["state_reason"] != "completed" {
            return Err("Auto recovery requires the completed Auto Issue".into());
        }
        let accepted = [
            issue::body(provider.repository),
            issue::legacy(provider.repository),
        ];
        if !accepted
            .iter()
            .any(|body| ticket["body"] == body.replace("- [ ]", "- [x]"))
        {
            return Err("Auto recovery requires the accepted Auto Issue body".into());
        }
        if provider.remote(number)?.is_some() {
            return Err("Auto recovery requires the remote branch to be absent".into());
        }
        let pull = provider
            .find(number)?
            .ok_or("Auto recovery cannot find the registered pull")?;
        let observed = provider.pull(state.pull)?;
        let binding = |value: &serde_json::Value| {
            ["node_id", "number", "head", "base", "merge_commit_sha"].map(|key| value[key].clone())
        };
        if binding(&pull) != binding(&observed) || pull["number"].as_u64() != Some(state.pull) {
            return Err("Auto recovery pull identity disagrees".into());
        }
        if observed["state"] != "closed"
            || observed["merged"] != true
            || !observed["merged_at"].is_string()
        {
            return Err("Auto recovery requires a verified merged pull".into());
        }
        let head = issue::text(&observed["head"], "sha")?;
        let merge = issue::text(&observed, "merge_commit_sha")?;
        let history = provider.intents(number)?;
        if observed["base"]["repo"]["full_name"] != provider.repository
            || history
                .last()
                .is_none_or(|intent| observed["base"]["sha"] != intent.base)
        {
            return Err("Auto recovery pull base disagrees with publication intent".into());
        }
        let candidate = self.verify(&issue, &head, &merge, &history)?;
        let guard = provider.guard(&head)?;
        let accepted = provider.evidence(number, "<!-- plumb.auto-closure/", "Auto closure")?;
        if accepted
            != [closure::acceptance(
                provider.repository,
                &merge,
                &guard,
                &head,
            )]
        {
            return Err("Auto recovery lacks exact writer-owned closure evidence".into());
        }
        let main = work::git(self.root, &["rev-parse", "origin/main"])?;
        work::git(self.root, &["merge-base", "--is-ancestor", &merge, &main])?;
        Ok(Evidence {
            issue,
            head,
            merge,
            guard,
            candidate,
            history,
            main,
        })
    }

    fn verify(
        &self,
        issue: &plumb::delivery::Snapshot,
        head: &str,
        merge: &str,
        history: &[Intent],
    ) -> Result<plumb::guard::Descriptor, String> {
        let state = self.state;
        let plan = state
            .plan
            .as_ref()
            .ok_or("Auto recovery has no recorded plan")?;
        let pushed = state
            .pushed
            .as_deref()
            .ok_or("Auto recovery has no recorded push")?;
        let expected = (state.repository.as_str(), issue.node.as_str(), issue.number);
        let actual = (
            plan.repository.as_str(),
            plan.issue.node.as_str(),
            plan.issue.number,
        );
        if actual != expected
            || (plan.base.as_str(), plan.branch.as_str())
                != ("main", &format!("auto/{}", issue.number))
        {
            return Err("Auto recovery plan identity disagrees".into());
        }
        if state.candidate.as_deref() != Some(&plan.candidate) || plan.candidate == head {
            return Err(
                "Auto recovery requires a distinct, consistently recorded candidate".into(),
            );
        }
        if state.merged.is_some() || state.guard.is_some() {
            return Err("Auto recovery refuses recorded merge or Guard settlement".into());
        }
        chain(history, pushed, head, plan)?;
        work::git(
            self.root,
            &[
                "fetch",
                "--no-tags",
                "origin",
                "refs/heads/main:refs/remotes/origin/main",
            ],
        )?;
        work::git(
            self.root,
            &[
                "fetch",
                "--no-tags",
                "origin",
                &format!("refs/pull/{}/head", state.pull),
            ],
        )?;
        if work::git(self.root, &["rev-parse", "FETCH_HEAD"])? != head {
            return Err("Auto recovery pull head moved during acquisition".into());
        }
        for intent in history {
            self.publication(intent)?;
        }
        geometry(self.root, &plan.candidate, &plan.target, &plan.guard.tree)?;
        let old = plumb::guard::commit(self.root, &plan.candidate)?;
        let landed =
            plumb::delivery::landed(self.root, head, merge).map_err(|error| error.to_string())?;
        geometry(self.root, merge, &plan.target, &plan.guard.tree)?;
        if (&old.schema, &old.digest, &old.tree)
            != (&plan.guard.schema, &plan.guard.digest, &plan.guard.tree)
        {
            return Err("Auto recovery recorded candidate proof disagrees".into());
        }
        scope::check(self.root, &landed.parent, head)?;
        Ok(old)
    }

    fn publication(&self, intent: &Intent) -> Result<(), String> {
        match &self.provider.source {
            Source::Native(client) => self.native(client, intent),
            #[cfg(all(test, unix))]
            Source::Fixture => geometry(self.root, &intent.head, &intent.base, &intent.tree),
        }
    }

    fn native(&self, client: &runseal::github::Client, intent: &Intent) -> Result<(), String> {
        let (owner, repo) = self.provider.coordinate()?;
        let commit = client
            .commit(owner, repo, &intent.head)
            .map_err(crate::command::diagnostic)?
            .value;
        if commit.tree.sha != intent.tree
            || commit
                .parents
                .iter()
                .map(|parent| parent.sha.as_str())
                .ne([intent.base.as_str()])
        {
            return Err("Auto recovery publication Git parent or tree disagrees".into());
        }
        Ok(())
    }

    pub(super) fn local(&self, seat: &Seat, evidence: &Evidence) -> Result<String, String> {
        let state = self.state;
        let worktree = &state.worktree;
        if !seat.owns(state)?
            || !std::fs::symlink_metadata(worktree).is_ok_and(|metadata| metadata.is_dir())
        {
            return Err("Auto recovery requires the existing owned worktree".into());
        }
        work::Work {
            source: self.root,
            state,
        }
        .inspect()?;
        let status = work::git(
            worktree,
            &["status", "--porcelain", "--untracked-files=all"],
        )?;
        if !status.is_empty() {
            return Err("Auto recovery preserves uncommitted local payload".into());
        }
        let local = work::git(worktree, &["rev-parse", "HEAD"])?;
        let plan = state
            .plan
            .as_ref()
            .ok_or("Auto recovery has no recorded plan")?;
        if local != plan.source
            && local != plan.candidate
            && !evidence.history.iter().any(|intent| intent.head == local)
        {
            return Err("Auto recovery local branch moved outside the recorded history".into());
        }
        if work::git(worktree, &["rev-parse", "HEAD^{tree}"])? != plan.guard.tree {
            return Err("Auto recovery local branch tree disagrees".into());
        }
        Ok(local)
    }
}

fn chain(
    history: &[Intent],
    pushed: &str,
    head: &str,
    plan: &plumb::delivery::Plan,
) -> Result<(), String> {
    let mut previous = None;
    let mut seen = BTreeSet::new();
    for intent in history {
        if intent.previous.as_deref() != previous || !seen.insert(intent.head.as_str()) {
            return Err("Auto recovery publication chain is discontinuous".into());
        }
        if (&intent.base, &intent.tree) != (&plan.target, &plan.guard.tree) {
            return Err("Auto recovery publication chain changed tree or base".into());
        }
        previous = Some(intent.head.as_str());
    }
    if !seen.contains(pushed) || pushed == head || previous != Some(head) {
        return Err("Auto recovery publication chain must supersede the recorded push with the exact merged head".into());
    }
    Ok(())
}

fn geometry(root: &Path, head: &str, base: &str, tree: &str) -> Result<(), String> {
    let commit = work::git(root, &["show", "-s", "--format=%H %P %T", head])?;
    if commit != format!("{head} {base} {tree}") {
        return Err("Auto recovery Git commit, parent or tree disagrees".into());
    }
    Ok(())
}
