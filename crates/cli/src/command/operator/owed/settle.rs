use super::super::consign::{Consign, brief, consign};
use super::super::wharf::{git, repository, text};
use super::plan::{State, target};
use super::{Obligation, Seat, observed, said};
use crate::shape::release::Spec;
use plumb::land::rejoin::Stable;
use serde::Serialize;
use std::path::PathBuf;

pub struct Settle {
    pub version: Option<String>,
    pub changelog: Option<PathBuf>,
    pub skill: Option<PathBuf>,
    pub remote: String,
    pub dry: bool,
    pub json: bool,
}

#[derive(Serialize)]
struct Receipt {
    version: String,
    commit: String,
    actions: Vec<String>,
    owed: Vec<String>,
    settled: bool,
}

struct Outcome<'a> {
    spec: &'a Spec,
    stable: &'a Stable,
    statuses: &'a [(&'a Obligation, bool)],
}

impl Outcome<'_> {
    fn render(&self, actions: Vec<String>, json: bool) -> Result<String, String> {
        let owed = self
            .statuses
            .iter()
            .filter(|(_, done)| !done)
            .map(|(obligation, _)| obligation.name.to_string())
            .collect::<Vec<_>>();
        if json {
            return serde_json::to_string_pretty(&Receipt {
                version: self.stable.marker.clone(),
                commit: self.stable.commit.clone(),
                actions,
                settled: owed.is_empty(),
                owed,
            })
            .map_err(|error| format!("cannot encode settle receipt: {error}"));
        }
        let home = plumb::config::home();
        let mut lines = actions;
        lines.push(said(
            self.stable,
            self.statuses,
            observed(self.spec, &self.stable.marker, home.as_deref()),
        ));
        Ok(lines.join("\n"))
    }
}

pub(in crate::command) fn settle(options: Settle) -> Result<String, String> {
    let root = super::super::worktree::root()?;
    let remote = options.remote.as_str();
    let url = text(
        "read the remote",
        git(&root, &["remote", "get-url", remote])?,
    )?;
    repository(&url)?;
    let list = || {
        text(
            "list the remote",
            git(&root, &["ls-remote", "--heads", "--tags", remote])?,
        )
    };
    let listing = list()?;
    let spec = Spec::controller(&root)?;
    let stable = target(&listing, options.version.as_deref())?;
    let seat = Seat {
        root: &root,
        remote,
        listing: &listing,
        spec: &spec,
    };
    let statuses = seat.statuses(&stable)?;
    let plan =
        State::from(&spec, &statuses).plan(&stable.marker, options.changelog, options.skill)?;
    if let Some(dir) = &plan.changelog {
        crate::command::changelog::prove(&root, dir, &stable.marker)?;
    }
    if let Some(dir) = &plan.skill {
        brief(dir)?;
    }
    if options.dry {
        return Outcome {
            spec: &spec,
            stable: &stable,
            statuses: &statuses,
        }
        .render(plan.steps(&stable.marker), options.json);
    }
    let mut done = run(plan.jobs(&stable.marker))?;
    if plan.close {
        done.push(super::super::line::close(
            &stable.marker,
            false,
            remote,
            false,
        )?);
    }
    let fresh = list()?;
    let updated = Seat {
        root: &root,
        remote,
        listing: &fresh,
        spec: &spec,
    }
    .statuses(&stable)?;
    Outcome {
        spec: &spec,
        stable: &stable,
        statuses: &updated,
    }
    .render(done, options.json)
}

fn run(jobs: Vec<Consign>) -> Result<Vec<String>, String> {
    if jobs.is_empty() {
        return Ok(Vec::new());
    }
    let results = std::thread::scope(|scope| {
        let handles = jobs
            .into_iter()
            .map(|job| scope.spawn(move || consign(job)))
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .map(|handle| {
                handle
                    .join()
                    .unwrap_or_else(|_| Err("consign worker panicked".into()))
            })
            .collect::<Vec<_>>()
    });
    results.into_iter().collect()
}

pub(in crate::command::operator) fn after(seat: &Seat<'_>, marker: &str, commit: &str) -> String {
    if crate::command::release::channel(marker).as_deref() != Ok("stable") {
        return String::new();
    }
    let stable = Stable {
        marker: marker.to_string(),
        commit: commit.to_string(),
    };
    let Ok(statuses) = seat.statuses(&stable) else {
        return String::new();
    };
    let pending = statuses
        .iter()
        .filter(|(_, done)| !done)
        .map(|(obligation, _)| obligation.name)
        .collect::<Vec<_>>();
    if pending.is_empty() {
        return String::new();
    }
    let mut hint = format!("plumb release settle --version {marker} --changelog <notes>");
    if seat.spec.skill {
        hint.push_str(" --skill <brief>");
    }
    format!("\nremaining owed: {}\nnext: {hint}", pending.join(", "))
}

pub(in crate::command::operator) fn remaining(seat: &Seat<'_>, version: &str) -> Option<String> {
    let stable = target(seat.listing, Some(version)).ok()?;
    let statuses = seat.statuses(&stable).ok()?;
    let pending = statuses
        .iter()
        .filter(|(obligation, done)| !done && obligation.name != "its release line closed")
        .map(|(obligation, _)| {
            format!(
                "  owed     {} [{}]: run {}",
                obligation.name,
                obligation.rule.0,
                obligation.hint(&stable)
            )
        })
        .collect::<Vec<_>>();
    (!pending.is_empty()).then(|| format!("still owed for {version}:\n{}", pending.join("\n")))
}
