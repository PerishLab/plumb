use super::{Provider, Source, encode};
use crate::command::diagnostic;
use runseal::github::{Number, Page, Relation, State, Window, lifecycle, records::Draft};
use serde::Serialize;
use serde_json::Value;
#[cfg(all(test, unix))]
use serde_json::json;
use std::collections::BTreeSet;

impl Provider<'_> {
    pub(in super::super) fn merge(
        &self,
        number: u64,
        squash: &plumb::delivery::Squash,
    ) -> Result<(), String> {
        match &self.source {
            Source::Native(client) => {
                plumb::github::merge(&self.target(number)?, client, squash, "Auto")
            }
            #[cfg(all(test, unix))]
            Source::Fixture => self.settle(|settle| settle.merge(number, squash)),
        }
    }

    pub(in super::super) fn raise(&self, body: &str) -> Result<u64, String> {
        let title = "Follow the latest first-party packages";
        match &self.source {
            Source::Native(client) => {
                let (owner, name) = self.coordinate()?;
                let draft = Draft::new(title, body, "Auto").map_err(diagnostic)?;
                let issue = client
                    .file(owner, name, &draft)
                    .map_err(|error| {
                        format!("create repository Auto Issue: {}", diagnostic(error))
                    })?
                    .value;
                Ok(issue.number.into())
            }
            #[cfg(all(test, unix))]
            Source::Fixture => {
                let endpoint = format!("repos/{}/issues", self.repository);
                let input = json!({"title": title, "type": "Auto", "body": body});
                let reply = self.api(&endpoint, Some(&input))?;
                reply["number"]
                    .as_u64()
                    .ok_or("created Auto issue has no number; reread before retry".into())
            }
        }
    }

    pub(in super::super) fn discussion(&self, number: u64) -> Result<Vec<Value>, String> {
        match &self.source {
            Source::Native(client) => {
                let target = self.target(number)?;
                numbered(|window| {
                    let page = client
                        .discussion(&target, window)
                        .map_err(diagnostic)?
                        .value;
                    Ok((page.rows, page.next))
                })
            }
            #[cfg(all(test, unix))]
            Source::Fixture => self.pages(&format!(
                "repos/{}/issues/{number}/comments?per_page=100",
                self.repository
            )),
        }
    }

    pub(in super::super) fn unresolved(
        &self,
        number: u64,
        relation: Relation,
    ) -> Result<bool, String> {
        match &self.source {
            Source::Native(client) => {
                let target = self.target(number)?;
                let rows =
                    cursors(|window| client.relations(&target, window, relation).map(|r| r.value))?;
                Ok(rows.iter().any(|row| row.state != "CLOSED"))
            }
            #[cfg(all(test, unix))]
            Source::Fixture => {
                let path = match relation {
                    Relation::Child => "sub_issues",
                    Relation::Blocker => "dependencies/blocked_by",
                };
                let rows = self.pages(&format!(
                    "repos/{}/issues/{number}/{path}?per_page=100",
                    self.repository
                ))?;
                Ok(rows.iter().any(|row| row["state"] != "closed"))
            }
        }
    }

    pub(in super::super) fn complete(&self, number: u64, body: &str) -> Result<(), String> {
        match &self.source {
            Source::Native(client) => {
                let issue = self.issue(number)?;
                client
                    .complete(&super::super::issue::text(&issue, "node_id")?, body)
                    .map(|_| ())
                    .map_err(diagnostic)
            }
            #[cfg(all(test, unix))]
            Source::Fixture => self
                .patch(
                    &format!("repos/{}/issues/{number}", self.repository),
                    &json!({"body": body, "state": "closed"}),
                )
                .map(|_| ()),
        }
    }

    pub(in super::super) fn listing(
        &self,
        branch: &str,
        base: Option<&str>,
    ) -> Result<Vec<Value>, String> {
        match &self.source {
            Source::Native(client) => numbered(|window| {
                let (owner, name) = self.coordinate()?;
                let selection = lifecycle::Selection::new(branch, base, State::All, window)
                    .map_err(diagnostic)?;
                let page = client
                    .listing(owner, name, &selection)
                    .map_err(diagnostic)?
                    .value;
                Ok((page.rows, page.next))
            }),
            #[cfg(all(test, unix))]
            Source::Fixture => {
                let (owner, _) = self.coordinate()?;
                let base = base.map_or_else(String::new, |base| format!("&base={base}"));
                self.pages(&format!(
                    "repos/{}/pulls?state=all&head={owner}:{branch}{base}&per_page=100",
                    self.repository
                ))
            }
        }
    }

    pub(in super::super) fn revise(
        &self,
        number: u64,
        update: &lifecycle::Update,
    ) -> Result<Value, String> {
        match &self.source {
            Source::Native(client) => encode(
                client
                    .revise(&self.target(number)?, update)
                    .map_err(diagnostic)?
                    .value,
            ),
            #[cfg(all(test, unix))]
            Source::Fixture => {
                let (field, value) = match update {
                    lifecycle::Update::Body(body) => ("body", body.as_str()),
                    lifecycle::Update::Close => ("state", "closed"),
                };
                let body = json!({field: value});
                self.patch(&format!("repos/{}/pulls/{number}", self.repository), &body)
            }
        }
    }
}

fn numbered<T: Serialize>(
    mut read: impl FnMut(Number) -> Result<(Vec<T>, Option<u32>), String>,
) -> Result<Vec<Value>, String> {
    let mut rows = Vec::new();
    let mut page = 1;
    let mut seen = BTreeSet::new();
    for _ in 0..100 {
        let (entries, next) = read(Number::new(page, 100).map_err(diagnostic)?)?;
        for entry in entries {
            let value = encode(entry)?;
            let id = value["id"]
                .as_u64()
                .ok_or("provider row has no numeric identity")?;
            if !seen.insert(id) {
                return Err("provider returned the same identity across pages".into());
            }
            rows.push(value);
        }
        match next {
            Some(next) => page = next,
            None => return Ok(rows),
        }
    }
    Err("provider did not complete its numbered listing within 100 pages".into())
}

fn cursors<T>(
    mut read: impl FnMut(&Window) -> Result<Page<T>, runseal::github::Error>,
) -> Result<Vec<T>, String> {
    let mut rows = Vec::new();
    let mut cursor = None;
    let mut seen = BTreeSet::new();
    let mut scope = None;
    for _ in 0..100 {
        let window = Window::new(100, cursor.as_deref()).map_err(diagnostic)?;
        let page = read(&window).map_err(diagnostic)?;
        let current = (page.target, page.total as usize);
        if scope.get_or_insert(current.clone()) != &current {
            return Err("provider scope or total changed across pages".into());
        }
        rows.extend(page.rows);
        match page.next {
            Some(next) if seen.insert(next.clone()) => cursor = Some(next),
            None if rows.len() == current.1 => return Ok(rows),
            _ => return Err("provider returned inconsistent pagination".into()),
        }
    }
    Err("provider did not complete its cursor listing within 100 pages".into())
}
