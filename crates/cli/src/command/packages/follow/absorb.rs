use super::{Provider, Seat, State, issue, scope, work};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::Path;

const MARKER: &str = "<!-- plumb.auto-absorbed/v1\n";

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Receipt {
    pub schema: String,
    pub repository: String,
    pub issue: u64,
    pub pull: u64,
    pub head: String,
    pub main: String,
    pub resolution: plumb::packages::Resolution,
}

pub(super) struct Absorption<'a, 'p> {
    pub root: &'a Path,
    pub provider: &'a Provider<'p>,
    pub seat: &'a Seat,
}

impl Absorption<'_, '_> {
    pub(super) fn absorb(&self, state: &mut State) -> Result<String, String> {
        let head = state
            .pushed
            .clone()
            .ok_or("absorbed Auto pull has no recorded push")?;
        if state.merged.is_some()
            || state.guard.is_some()
            || state
                .plan
                .as_ref()
                .is_some_and(|plan| plan.candidate != head)
        {
            return Err("absorbed Auto pull has delivery progress beyond its recorded push".into());
        }
        if issue::snapshot(self.provider, state.issue)?.state != "OPEN" {
            return Err("absorbed Auto settlement requires an open Auto issue".into());
        }
        let pull = self.own(state, &head)?;
        if pull["state"] != "open" {
            return Err("absorbed Auto pull is not open".into());
        }
        self.published(state, &head)?;
        let receipt = self.observe(state, head)?;
        let body = format!(
            "Auto follow absorbed. Current main: https://github.com/{}/commit/{}. The Auto's own pull #{} at exact head {} contributes no change beyond main: latest-stable resolution on main has no registered delta. The pull is closed without merge and its owned branch retired by exact lease. This Auto produced no merge or Guard.\n\n{MARKER}{}\n-->",
            self.provider.repository,
            receipt.main,
            receipt.pull,
            receipt.head,
            serde_json::to_string(&receipt).map_err(|error| error.to_string())?
        );
        self.provider.comment(state.issue, &body)?;
        if self.provider.absorption(state.issue)?.as_ref() != Some(&receipt) {
            return Err("absorbed Auto receipt readback disagrees".into());
        }
        self.settle(state)
    }

    fn published(&self, state: &State, head: &str) -> Result<(), String> {
        let intent = self
            .provider
            .recorded(state.issue)?
            .ok_or("absorbed Auto pull has no publication intent")?;
        if intent.head != head || self.provider.remote(state.issue)?.as_deref() != Some(head) {
            return Err("absorbed Auto pull moved outside its recorded publication".into());
        }
        if work::git(self.root, &["rev-parse", &format!("{head}^{{tree}}")])? != intent.tree {
            return Err("absorbed Auto publication tree disagrees".into());
        }
        scope::check(self.root, &intent.base, head)
    }

    fn observe(&self, state: &State, head: String) -> Result<Receipt, String> {
        let main = work::git(self.root, &["rev-parse", "origin/main"])?;
        let resolution = plumb::packages::locked(&state.worktree, "follow")?;
        plumb::packages::Plan::read(&state.worktree, "follow")?.verify(&state.worktree)?;
        work::current(self.root, &main)?;
        if resolution.tree != work::git(self.root, &["rev-parse", &format!("{main}^{{tree}}")])? {
            return Err("absorbed Auto resolution differs from current main".into());
        }
        Ok(Receipt {
            schema: "plumb.auto-absorbed/v1".into(),
            repository: self.provider.repository.into(),
            issue: state.issue,
            pull: state.pull,
            head,
            main,
            resolution,
        })
    }

    pub(super) fn settle(&self, state: &mut State) -> Result<String, String> {
        let receipt = self
            .provider
            .absorption(state.issue)?
            .ok_or("Auto has no absorption receipt")?;
        if (receipt.repository.as_str(), receipt.issue, receipt.pull)
            != (self.provider.repository, state.issue, state.pull)
            || receipt.resolution.context != "follow"
        {
            return Err("Auto absorption receipt identity disagrees".into());
        }
        work::git(
            self.root,
            &["merge-base", "--is-ancestor", &receipt.main, "origin/main"],
        )?;
        if work::git(
            self.root,
            &["rev-parse", &format!("{}^{{tree}}", receipt.main)],
        )? != receipt.resolution.tree
        {
            return Err("Auto absorption receipt main tree disagrees".into());
        }
        self.close(state, &receipt.head)?;
        self.retire(&receipt)?;
        work::Work {
            source: self.root,
            state,
        }
        .retire(&receipt.resolution.tree)?;
        self.accept(state.issue)?;
        *state = State {
            schema: "plumb.auto-state/v1".into(),
            repository: self.provider.repository.into(),
            ..State::default()
        };
        self.seat.write(state)?;
        Ok(format!(
            "Auto #{} settled: its own pull #{} at {} was absorbed by main {}; no merge or Guard produced",
            receipt.issue, receipt.pull, receipt.head, receipt.main
        ))
    }

    fn own(&self, state: &State, head: &str) -> Result<Value, String> {
        let pull = self
            .provider
            .find(state.issue)?
            .ok_or("absorbed Auto pull is missing")?;
        if pull["number"].as_u64() != Some(state.pull)
            || pull["head"]["sha"] != head
            || !pull["merged_at"].is_null()
        {
            return Err("absorbed Auto pull is not its own exact unmerged head".into());
        }
        Ok(pull)
    }

    fn close(&self, state: &State, head: &str) -> Result<(), String> {
        if self.own(state, head)?["state"] == "open" {
            self.provider.patch(
                &format!("repos/{}/pulls/{}", self.provider.repository, state.pull),
                &json!({"state":"closed"}),
            )?;
        }
        if self.own(state, head)?["state"] != "closed" {
            return Err("absorbed Auto pull closure was not read back".into());
        }
        Ok(())
    }

    fn retire(&self, receipt: &Receipt) -> Result<(), String> {
        match self.provider.remote(receipt.issue)? {
            Some(head) if head == receipt.head => {
                let branch = format!("refs/heads/auto/{}", receipt.issue);
                work::git(
                    self.root,
                    &[
                        "push",
                        &format!("--force-with-lease={branch}:{head}"),
                        "origin",
                        &format!(":{branch}"),
                    ],
                )?;
            }
            Some(_) => return Err("absorbed Auto branch moved outside its receipt".into()),
            None => {}
        }
        if self.provider.remote(receipt.issue)?.is_some() {
            return Err("absorbed Auto branch retirement was not read back".into());
        }
        Ok(())
    }

    fn accept(&self, number: u64) -> Result<(), String> {
        let current = self.provider.issue(number)?;
        if current["state"] != "closed" {
            let accepted = issue::text(&current, "body")?.replace("- [ ]", "- [x]");
            self.provider.patch(
                &format!("repos/{}/issues/{number}", self.provider.repository),
                &json!({"body": accepted, "state": "closed"}),
            )?;
        }
        if self.provider.issue(number)?["state"] != "closed" {
            return Err("absorbed Auto issue closure was not read back".into());
        }
        Ok(())
    }
}

impl Provider<'_> {
    pub(super) fn absorption(&self, number: u64) -> Result<Option<Receipt>, String> {
        let owner = self.issue(number)?["user"]["id"]
            .as_u64()
            .filter(|id| *id > 0)
            .ok_or("Auto issue has no operation writer identity")?;
        let comments = self.pages(&format!(
            "repos/{}/issues/{number}/comments?per_page=100",
            self.repository
        ))?;
        let mut found = None;
        for comment in comments {
            let body = issue::text(&comment, "body")?;
            let Some((_, tail)) = body.split_once(MARKER) else {
                continue;
            };
            if comment["user"]["id"].as_u64() != Some(owner) {
                return Err("Auto absorption receipt has another operation writer".into());
            }
            let value = tail
                .strip_suffix("\n-->")
                .ok_or("Auto absorption receipt has an unknown shape")?;
            let held: Receipt = serde_json::from_str(value)
                .map_err(|error| format!("Auto absorption receipt is unreadable: {error}"))?;
            if held.schema != "plumb.auto-absorbed/v1" {
                return Err("Auto absorption receipt schema disagrees".into());
            }
            found = Some(held);
        }
        Ok(found)
    }
}
