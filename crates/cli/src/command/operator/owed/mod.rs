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

pub const OBLIGATIONS: [Obligation; 3] = [
    Obligation {
        rule: &law::STABLE_REJOIN,
        name: "its commit merged home into main",
        settle: &["release", "rejoin"],
        arguments: |_| String::new(),
        applies: |_| true,
        detect: rejoined,
    },
    Obligation {
        rule: &law::STABLE_LODGED,
        name: "its changelog lodged on Depot",
        settle: &["depot", "consign"],
        arguments: |stable| format!("--version {} --kind changelog --dir <notes>", stable.marker),
        applies: |_| true,
        detect: |seat, stable| lodged(seat, stable, Kind::Changelog),
    },
    Obligation {
        rule: &law::STABLE_LODGED,
        name: "its skill lodged on Depot",
        settle: &["depot", "consign"],
        arguments: |stable| format!("--version {} --kind skill", stable.marker),
        applies: |spec| spec.skill,
        detect: |seat, stable| lodged(seat, stable, Kind::Skill),
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

pub fn require(seat: &Seat<'_>, marker: &str) -> Result<(), String> {
    let Some(stable) = below(seat.listing, marker) else {
        return Ok(());
    };
    let mut owed = Vec::new();
    for obligation in OBLIGATIONS.iter().filter(|held| (held.applies)(seat.spec)) {
        if !(obligation.detect)(seat, &stable)? {
            owed.push(format!(
                "{} [{}] (run {})",
                obligation.name,
                obligation.rule.0,
                obligation.hint(&stable)
            ));
        }
    }
    if owed.is_empty() {
        return Ok(());
    }
    Err(format!(
        "stable {} still owes {}; settle it before {marker}",
        stable.marker,
        owed.join(", and ")
    ))
}

pub fn source(spec: &Spec) -> String {
    spec.route
        .clone()
        .unwrap_or_else(|| format!("https://depot.{}.perish.uk", spec.product))
}

pub fn rejoined(seat: &Seat<'_>, stable: &Stable) -> Result<bool, String> {
    let main = seat
        .listing
        .lines()
        .find_map(|line| {
            let (object, name) = line.split_once('\t')?;
            (name == "refs/heads/main").then(|| object.to_string())
        })
        .ok_or_else(|| {
            format!(
                "{} has no main; a stable marker settles into main",
                seat.remote
            )
        })?;
    let reference = format!("refs/tags/{}", stable.marker);
    let output = Command::new("git")
        .args([
            "fetch",
            "--no-tags",
            seat.remote,
            "refs/heads/main",
            &reference,
        ])
        .current_dir(seat.root)
        .output()
        .map_err(|error| format!("cannot run git: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cannot fetch main and {}: {}",
            stable.marker,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(rejoin::settled(seat.root, &stable.commit, &main))
}

fn lodged(seat: &Seat<'_>, stable: &Stable, kind: Kind) -> Result<bool, String> {
    let source = source(seat.spec);
    Generation::latest(Query {
        source: &source,
        product: &seat.spec.product,
        channel: "stable",
        version: &stable.marker,
        kind,
    })
    .map(|found| found.is_some())
    .map_err(|error| format!("cannot read {source} for {}: {error}", stable.marker))
}

#[cfg(test)]
mod proof;
