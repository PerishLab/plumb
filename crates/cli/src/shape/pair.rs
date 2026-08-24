use super::{Found, Shape};
use crate::catalog::rules::release as release_rule;
use crate::catalog::rules::structure as rule;
use crate::judge::finding::Seed;
use crate::judge::show;

mod identity;
mod read;

pub use read::{Release, Root};

fn measured(release: &Release, found: &mut Found) {
    let rules = &crate::catalog::set::RULES.release;
    for (attachment, held) in &release.widths {
        let permitted = rules
            .permitted
            .get(attachment)
            .copied()
            .unwrap_or(rules.ceiling);
        if *held > permitted {
            found.push(Seed::wrong(
                &release_rule::ATTACHMENT_PERMITTED,
                format!(
                    "the {attachment} attachment declares {held} packages and Plumb permits {permitted}; widening it is a change to Plumb"
                ),
            ));
            continue;
        }
        let exercised = rules.exercised.get(attachment).copied().unwrap_or(0);
        if *held > exercised {
            found.push(Seed::noted(
                &release_rule::ATTACHMENT_EXERCISED,
                format!(
                    "the {attachment} attachment declares {held} packages and the skeleton has released {exercised}; this width has never run"
                ),
            ));
        }
    }
}

fn carriers(attachment: &str) -> &'static [&'static str] {
    match attachment {
        "binary" | "skill" | "deb" | "npm" | "oci" | "chart" => &["release-binary", "ship"],
        "cargo" => &["release-binary", "release-cargo", "ship"],
        _ => &[],
    }
}

pub fn judge(held: &Shape) -> Found {
    Judge(held).run()
}

struct Judge<'a>(&'a Shape);

impl Judge<'_> {
    fn run(&self) -> Found {
        let held = self.0;
        let mut found = Found::new();
        if held.rust && !held.ignore.lines().any(|line| line.trim() == "target/") {
            found.push(Seed::wrong(
                &rule::CARGO_TARGET_IGNORED,
                "Cargo.toml without target/ in .gitignore".to_string(),
            ));
        }
        spec(&held.release, &mut found);
        measured(&held.release, &mut found);
        deliverable(&held.release, &mut found);
        if held.ships.contains("binary") {
            self.anchors(&mut found);
        }
        self.site(&mut found);
        found
    }

    fn anchors(&self, found: &mut Found) {
        let held = self.0;
        if held.lanes.contains("exact.release") && held.lanes.contains("stable.release") {
            return;
        }
        for lane in ["release-exact", "release-stable"] {
            if !held.lanes.contains(lane) {
                found.push(Seed::wrong(
                    &rule::RELEASE_LANE_PRESENT,
                    format!("binary release without a {lane} lane"),
                ));
            } else if !held.release.sources.contains(lane) {
                found.push(Seed::wrong(
                    &rule::RELEASE_SOURCE_BOUND,
                    format!("{lane} exposes or forwards a second source binding"),
                ));
            }
        }
    }

    fn site(&self, found: &mut Found) {
        let held = self.0;
        if held.sites.is_empty() || held.lanes.contains("deploy") {
            return;
        }
        if held.ships.contains("cfworker") && held.lanes.contains("ship") {
            return;
        }
        found.push(Seed::wrong(
            &rule::SITE_DEPLOY_LANE,
            format!(
                "{} declares a site that no lane delivers",
                show(&held.sites)
            ),
        ));
    }
}

fn spec(release: &Release, found: &mut Found) {
    if let Some(refusal) = &release.refusal {
        found.push(Seed::wrong(
            &release_rule::SPEC_DECLARED,
            format!("plumb.toml declares a release the current Plumb refuses: {refusal}"),
        ));
    }
    if let Some(blind) = &release.blind {
        found.push(Seed::blind(&release_rule::SPEC_DECLARED, blind));
    }
}

fn deliverable(release: &Release, found: &mut Found) {
    if release.attachments.is_empty() {
        return;
    }
    if release.callers.contains("ship") {
        return;
    }
    for attachment in &release.attachments {
        let carriers = carriers(attachment);
        if carriers.is_empty() {
            found.push(Seed::wrong(
                &release_rule::ATTACHMENT_DELIVERABLE,
                format!(
                    "{attachment} attachment is declared and no shared release lane delivers it"
                ),
            ));
        } else if !carriers.iter().any(|lane| release.callers.contains(*lane)) {
            found.push(Seed::wrong(
                &release_rule::ATTACHMENT_DELIVERABLE,
                format!(
                    "{attachment} attachment is declared and this repository calls none of {}",
                    carriers.join(", ")
                ),
            ));
        }
    }
}
