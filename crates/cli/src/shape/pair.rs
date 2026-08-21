use super::{Found, Shape};
use crate::catalog::rules::release as release_rule;
use crate::catalog::rules::structure as rule;
use crate::command::release::model::Spec;
use crate::judge::finding::Seed;
use crate::judge::show;
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Default)]
pub struct Release {
    pub attachments: BTreeSet<String>,
    pub widths: std::collections::BTreeMap<String, usize>,
    pub refusal: Option<String>,
}

pub fn release(root: &Path) -> Release {
    let manifest = root.join("plumb.toml");
    if !manifest.is_file() || !seated(&manifest) {
        return Release::default();
    }
    match Spec::read(&manifest) {
        Ok(spec) => Release {
            attachments: attachments(&spec),
            widths: widths(&spec),
            refusal: None,
        },
        Err(refusal) => Release {
            attachments: BTreeSet::new(),
            widths: std::collections::BTreeMap::new(),
            refusal: Some(refusal),
        },
    }
}

fn seated(manifest: &Path) -> bool {
    std::fs::read_to_string(manifest)
        .ok()
        .map(|text| match text.parse::<toml::Table>() {
            Ok(table) => table.contains_key("release"),
            Err(_) => true,
        })
        .unwrap_or(false)
}

fn attachments(spec: &Spec) -> BTreeSet<String> {
    let mut found: BTreeSet<String> = spec.surface().into_iter().map(str::to_string).collect();
    for (present, name) in [(spec.skill, "skill"), (spec.deb.is_some(), "deb")] {
        if present {
            found.insert(name.to_string());
        }
    }
    found
}

fn widths(spec: &Spec) -> std::collections::BTreeMap<String, usize> {
    let mut found = std::collections::BTreeMap::new();
    if let Some(cargo) = &spec.cargo {
        found.insert("cargo".to_string(), cargo.packages.len());
    }
    if let Some(npm) = &spec.npm {
        found.insert("npm".to_string(), npm.packages.len());
    }
    found
}

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

fn callers(root: &Path) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let Ok(entries) = std::fs::read_dir(root.join(".forgejo/workflows")) else {
        return found;
    };
    for entry in entries.flatten() {
        let Ok(text) = std::fs::read_to_string(entry.path()) else {
            continue;
        };
        for line in text.lines() {
            let Some((_, called)) = line.split_once("/.forgejo/workflows/") else {
                continue;
            };
            let name = called
                .split(['@', ' ', '\t'])
                .next()
                .unwrap_or_default()
                .trim_end_matches(".yml");
            if !name.is_empty() {
                found.insert(name.to_string());
            }
        }
    }
    found
}

pub fn sites(root: &Path) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let Ok(entries) = std::fs::read_dir(root.join("apps")) else {
        return found;
    };
    for entry in entries.flatten() {
        if entry.path().join("wrangler.jsonc").exists() {
            found.insert(entry.file_name().to_string_lossy().to_string());
        }
    }
    found
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
        spec(held.release.refusal.as_deref(), &mut found);
        measured(&held.release, &mut found);
        deliverable(&held.release, &held.root, &mut found);
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
            } else if !self.bound(lane) {
                found.push(Seed::wrong(
                    &rule::RELEASE_SOURCE_BOUND,
                    format!("{lane} exposes or forwards a second source binding"),
                ));
            }
        }
    }

    fn bound(&self, lane: &str) -> bool {
        std::fs::read_to_string(
            self.0
                .root
                .join(".forgejo/workflows")
                .join(format!("{lane}.yml")),
        )
        .map(|text| {
            let loose = [
                "source_ref:",
                "source_commit:",
                "${{ inputs.ref }}",
                "\n      ref:\n",
            ];
            !loose.iter().any(|value| text.contains(value))
        })
        .unwrap_or(false)
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

fn spec(refusal: Option<&str>, found: &mut Found) {
    if let Some(refusal) = refusal {
        found.push(Seed::wrong(
            &release_rule::SPEC_DECLARED,
            format!("plumb.toml declares a release the current Plumb refuses: {refusal}"),
        ));
    }
}

fn deliverable(release: &Release, root: &Path, found: &mut Found) {
    if release.attachments.is_empty() {
        return;
    }
    let called = callers(root);
    if called.contains("ship") {
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
        } else if !carriers.iter().any(|lane| called.contains(*lane)) {
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
