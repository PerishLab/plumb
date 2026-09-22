use crate::catalog::model::Mechanism;
use crate::catalog::rules::release as law;
use crate::shape::pair::rejoin;
use crate::shape::release::Spec;
use plumb::depot::v3::{Generation, Kind, Query};
use plumb::land::rejoin::{Stable, tags};
use std::path::Path;
use std::process::Command;

pub struct Seat<'a> {
    pub root: &'a Path,
    pub remote: &'a str,
    pub listing: &'a str,
    pub spec: &'a Spec,
}

pub struct Obligation {
    pub rule: &'static Mechanism,
    pub name: &'static str,
    pub settle: &'static [&'static str],
    pub arguments: fn(&Stable) -> String,
    pub applies: fn(&Spec) -> bool,
    pub detect: fn(&Seat<'_>, &Stable) -> Result<bool, String>,
}

pub const OBLIGATIONS: [Obligation; 4] = [
    Obligation {
        rule: &law::STABLE_REJOIN,
        name: "its commit merged home into main",
        settle: &["release", "rejoin"],
        arguments: |_| String::new(),
        applies: |_| true,
        detect: |seat, stable| seat.rejoined(stable),
    },
    Obligation {
        rule: &law::STABLE_LODGED,
        name: "its changelog lodged on Depot",
        settle: &["depot", "consign"],
        arguments: |stable| format!("--version {} --kind changelog --dir <notes>", stable.marker),
        applies: |_| true,
        detect: |seat, stable| seat.lodged(stable, Kind::Changelog),
    },
    Obligation {
        rule: &law::STABLE_LODGED,
        name: "its skill lodged on Depot",
        settle: &["depot", "consign"],
        arguments: |stable| format!("--version {} --kind skill", stable.marker),
        applies: |spec| spec.skill,
        detect: |seat, stable| seat.lodged(stable, Kind::Skill),
    },
    Obligation {
        rule: &law::LINE_CLOSED,
        name: "its release line closed",
        settle: &["release", "close"],
        arguments: |stable| format!("--version {}", stable.marker),
        applies: |_| true,
        detect: |seat, stable| seat.closed(stable),
    },
];

impl Obligation {
    pub fn hint(&self, stable: &Stable) -> String {
        let command = format!("plumb {}", self.settle.join(" "));
        match (self.arguments)(stable) {
            held if held.is_empty() => command,
            held => format!("{command} {held}"),
        }
    }
}

pub fn below(listing: &str, marker: &str) -> Option<Stable> {
    let ceiling = semver::Version::parse(marker.strip_prefix('v')?).ok()?;
    plumb::land::rejoin::latest(tags(listing).into_iter().filter(|(name, _)| {
        name.strip_prefix('v')
            .and_then(|held| semver::Version::parse(held).ok())
            .is_some_and(|held| held < ceiling)
    }))
}

impl Seat<'_> {
    pub fn statuses<'a>(&self, stable: &Stable) -> Result<Vec<(&'a Obligation, bool)>, String> {
        let mut held = Vec::new();
        for obligation in OBLIGATIONS.iter().filter(|held| (held.applies)(self.spec)) {
            held.push((obligation, (obligation.detect)(self, stable)?));
        }
        Ok(held)
    }

    pub fn require(&self, marker: &str) -> Result<(), String> {
        let Some(stable) = below(self.listing, marker) else {
            return Ok(());
        };
        let owed = self
            .statuses(&stable)?
            .into_iter()
            .filter(|(_, settled)| !settled)
            .map(|(obligation, _)| {
                format!(
                    "{} [{}] (run {})",
                    obligation.name,
                    obligation.rule.0,
                    obligation.hint(&stable)
                )
            })
            .collect::<Vec<_>>();
        if owed.is_empty() {
            return Ok(());
        }
        Err(format!(
            "stable {} still owes {}; settle it before {marker}",
            stable.marker,
            owed.join(", and ")
        ))
    }

    pub fn report(&self, marker: Option<&str>) -> Result<String, String> {
        let stable = match marker {
            Some(marker) => below(self.listing, marker),
            None => plumb::land::rejoin::latest(tags(self.listing)),
        };
        let Some(stable) = stable else {
            return Ok("no stable marker stands below; nothing is owed".to_string());
        };
        let mut said = vec![format!("stable {} at {}", stable.marker, stable.commit)];
        for (obligation, settled) in self.statuses(&stable)? {
            let state = if settled { "settled" } else { "owed" };
            let mut line = format!("  {state:<8} {} [{}]", obligation.name, obligation.rule.0);
            if !settled {
                line.push_str(&format!(": run {}", obligation.hint(&stable)));
            }
            said.push(line);
        }
        Ok(said.join("\n"))
    }

    pub fn rejoined(&self, stable: &Stable) -> Result<bool, String> {
        let main = self
            .listing
            .lines()
            .find_map(|line| {
                let (object, name) = line.split_once('\t')?;
                (name == "refs/heads/main").then(|| object.to_string())
            })
            .ok_or_else(|| {
                format!(
                    "{} has no main; a stable marker settles into main",
                    self.remote
                )
            })?;
        let reference = format!("refs/tags/{}", stable.marker);
        let output = Command::new("git")
            .args([
                "fetch",
                "--no-tags",
                self.remote,
                "refs/heads/main",
                &reference,
            ])
            .current_dir(self.root)
            .output()
            .map_err(|error| format!("cannot run git: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "cannot fetch main and {}: {}",
                stable.marker,
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        Ok(rejoin::settled(self.root, &stable.commit, &main))
    }

    fn closed(&self, stable: &Stable) -> Result<bool, String> {
        let line = format!("refs/heads/release/{}", stable.marker);
        Ok(!self
            .listing
            .lines()
            .any(|held| held.split_once('\t').is_some_and(|(_, name)| name == line)))
    }

    fn lodged(&self, stable: &Stable, kind: Kind) -> Result<bool, String> {
        let source = source(self.spec);
        Generation::latest(Query {
            source: &source,
            product: &self.spec.product,
            channel: "stable",
            version: &stable.marker,
            kind,
        })
        .map(|found| found.is_some())
        .map_err(|error| format!("cannot read {source} for {}: {error}", stable.marker))
    }
}

pub fn source(spec: &Spec) -> String {
    spec.route
        .clone()
        .unwrap_or_else(|| format!("https://depot.{}.perish.uk", spec.product))
}

#[cfg(test)]
mod proof;
