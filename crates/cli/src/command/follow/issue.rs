use super::State;
use super::provider::{Provider, Source, encode};
use crate::command::diagnostic;
use runseal::github::{Client, Relation};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

const MARKER: &str = "<!-- plumb.auto/v1 operation=follow -->";
const RECOVERY: &str = "see: plumb cookbook follow.foreign-writer";

pub(super) fn find(provider: &Provider<'_>) -> Result<Option<u64>, String> {
    let mut matching = None;
    provider.open(|issue| {
        let marked = issue["body"]
            .as_str()
            .is_some_and(|body| body.contains(MARKER));
        if marked && issue.get("pull_request").is_none() && matching.replace(issue).is_some() {
            return Err("multiple open Auto follow issues require human judgment".into());
        }
        Ok(())
    })?;
    matching
        .map(|issue| {
            if issue["type"]["name"] != "Auto" {
                return Err("matching follow issue is not Auto; human judgment required".into());
            }
            issue["number"]
                .as_u64()
                .ok_or_else(|| "Auto issue has no number".into())
        })
        .transpose()
}

pub(super) fn create(provider: &Provider<'_>) -> Result<u64, String> {
    provider.raise(&body(provider.repository))
}

pub(super) fn body(repository: &str) -> String {
    legacy(repository)
        .replace("The bounded change is merged through the required organization Guard.", "Any bounded dependency change is merged through the required organization Guard, or a qualified no-op records no remaining delta and no associated pull.")
        .replace("Closure records the read-back merge commit and successful Guard run for the delivered change.", "Closure records the read-back merge and successful Guard for a delivered change, or exact current main, latest-stable resolution and owned empty-ref readback for a qualified no-op.")
}

pub(super) fn legacy(repository: &str) -> String {
    format!(
        "## Operation\nfollow\n\n## Target\n{repository}\n\n## Change\nSet first-party requirements to 0 and resolve crates from the perish registry and @perishlab npm packages to their latest stable. Allowed paths: Cargo.toml, package.json, Cargo.lock and pnpm-lock.yaml at the root or in repository packages. The pull request records the exact delta.\n\n## Acceptance\n- [ ] The bounded change is merged through the required organization Guard.\n- [ ] Closure records the read-back merge commit and successful Guard run for the delivered change.\n\n## Non-goals\nBusiness-source repair, unrelated third-party upgrades, releases and organization settings.\n\n{MARKER}\n"
    )
}

pub(super) fn snapshot(
    provider: &Provider<'_>,
    number: u64,
) -> Result<plumb::delivery::Snapshot, String> {
    let issue = provider.issue(number)?;
    if issue["type"]["name"] != "Auto"
        || !issue["body"].as_str().is_some_and(|body| {
            body.contains(MARKER) && body.contains(&format!("## Target\n{}\n", provider.repository))
        })
    {
        return Err("Issue is not this repository's registered Auto follow".into());
    }
    let accepted = [body(provider.repository), legacy(provider.repository)]
        .into_iter()
        .flat_map(|body| [body.replace("- [ ]", "- [x]"), body])
        .any(|body| issue["body"].as_str() == Some(body.as_str()));
    if !accepted {
        return Err("Auto follow body changed outside its operation contract".into());
    }
    if provider.unresolved(number, Relation::Child)? {
        return Err("Auto issue has an unresolved child".into());
    }
    for label in issue["labels"]
        .as_array()
        .ok_or("Issue labels are unreadable")?
    {
        if label["name"]
            .as_str()
            .is_some_and(|name| name.starts_with("needs:") || name.starts_with("acceptance:"))
        {
            return Err("Auto issue needs human resolution of its labels".into());
        }
    }
    if provider.unresolved(number, Relation::Blocker)? {
        return Err("Auto issue has an unresolved native blocker".into());
    }
    Ok(plumb::delivery::Snapshot {
        node: text(&issue, "node_id")?,
        repository: provider.repository.into(),
        number,
        url: text(&issue, "html_url")?,
        title: text(&issue, "title")?,
        state: text(&issue, "state")?.to_uppercase(),
        kind: "Auto".into(),
        updated: text(&issue, "updated_at")?,
        parent: None,
        sub_issues: Vec::new(),
        blocked_by: Vec::new(),
        blocking: Vec::new(),
    })
}

pub(super) fn text(value: &Value, key: &str) -> Result<String, String> {
    value[key]
        .as_str()
        .filter(|text| !text.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("provider field {key} is missing"))
}

pub(super) fn author(issue: &Value) -> Result<u64, String> {
    issue["user"]["id"]
        .as_u64()
        .filter(|id| *id > 0)
        .ok_or_else(|| "Auto issue has no operation writer identity".into())
}

impl Provider<'_> {
    pub(super) fn published(
        &self,
        previous: &plumb::delivery::Snapshot,
    ) -> Result<plumb::delivery::Snapshot, String> {
        let current = snapshot(self, previous.number)?;
        let mut expected = previous.clone();
        expected.updated = current.updated.clone();
        if current != expected {
            return Err("Auto Issue policy changed during candidate publication".into());
        }
        Ok(current)
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Noop {
    pub schema: String,
    pub repository: String,
    pub issue: u64,
    pub main: String,
    pub resolution: plumb::packages::Resolution,
    pub retired: Option<String>,
}

impl Provider<'_> {
    pub(super) fn completion(&self, number: u64) -> Result<Option<Noop>, String> {
        let mut found = None;
        for body in self.evidence(number, "<!-- plumb.auto-noop/", "Auto no-op receipt")? {
            let value = body
                .split_once("<!-- plumb.auto-noop/v1\n")
                .and_then(|(_, value)| value.strip_suffix("\n-->"))
                .ok_or("Auto no-op receipt has an unknown shape")?;
            found = Some(serde_json::from_str::<Noop>(value).map_err(|error| error.to_string())?);
        }
        Ok(found)
    }

    pub(super) fn owner(&self, issue: u64) -> Result<u64, String> {
        author(&self.issue(issue)?)
    }

    pub(super) fn writer(&self, issue: u64) -> Result<(), String> {
        let record = self.issue(issue)?;
        let owner = serde_json::from_value(record["user"].clone()).ok();
        plumb_cli::writer::check(issue, owner, || self.viewer())
    }

    pub(super) fn evidence(
        &self,
        issue: u64,
        marker: &str,
        what: &str,
    ) -> Result<Vec<String>, String> {
        let owner = self.owner(issue)?;
        let comments = self.discussion(issue)?;
        let (mut held, mut foreign) = (Vec::new(), Vec::new());
        for comment in comments {
            let body = comment["body"]
                .as_str()
                .ok_or("Auto comment is unreadable")?;
            if !body.contains(marker) {
                continue;
            }
            if comment["user"]["id"].as_u64() == Some(owner) {
                held.push(body.to_string());
            } else {
                foreign.push(body.to_string());
            }
        }
        if foreign.iter().any(|body| !held.contains(body)) {
            return Err(format!("{what} has another operation writer; {RECOVERY}"));
        }
        Ok(held)
    }

    pub(super) fn stopped(&self, state: &State, error: String) -> String {
        if state.issue == 0 {
            return format!(
                "{error}; no Auto issue or pull could be recorded; reread the provider before retry"
            );
        }
        if let Err(refusal) = self.writer(state.issue) {
            return if refusal == error {
                error
            } else {
                format!("{error}; {refusal}")
            };
        }
        let body = format!(
            "Auto follow stopped: {error}\n\nExisting pull: {}. Registered paths and required Guard remain unchanged. Resolve the blocker, then rerun plumb follow.",
            if state.pull == 0 {
                "none could be recorded; reread the provider before resuming".into()
            } else {
                format!("#{}", state.pull)
            }
        );
        if let Err(recording) = self.comment(state.issue, &body) {
            return format!("{error}; failure comment could not be recorded: {recording}");
        }
        error
    }
}

impl Provider<'_> {
    pub(super) fn open(
        &self,
        mut inspect: impl FnMut(Value) -> Result<(), String>,
    ) -> Result<(), String> {
        match &self.source {
            Source::Native(client) => self.inventory(client, &mut inspect),
            #[cfg(all(test, unix))]
            Source::Fixture => {
                for row in self.pages(&format!(
                    "repos/{}/issues?state=open&per_page=100",
                    self.repository
                ))? {
                    inspect(row)?;
                }
                Ok(())
            }
        }
    }

    fn inventory(
        &self,
        client: &Client,
        inspect: &mut impl FnMut(Value) -> Result<(), String>,
    ) -> Result<(), String> {
        let (owner, name) = self.coordinate()?;
        let (mut numbers, mut nodes) = (BTreeSet::new(), BTreeSet::new());
        let mut page = 1;
        for _ in 0..100 {
            let held = client
                .issues(owner, name, page, 30)
                .map_err(diagnostic)?
                .value;
            for row in held.rows {
                if !numbers.insert(row.number) || !nodes.insert(row.node.clone()) {
                    return Err("Auto Issue inventory repeated an Issue across pages".into());
                }
                inspect(encode(row)?)?;
            }
            match held.next {
                Some(next) => page = next,
                None => return Ok(()),
            }
        }
        Err("Auto Issue inventory exceeded its page limit".into())
    }
}
