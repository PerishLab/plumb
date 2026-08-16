use super::{Found, Shape};
use crate::dispatch::release::model::Spec;
use crate::judge::catalog::rules::release as release_rule;
use crate::judge::catalog::rules::structure as rule;
use crate::judge::finding::Seed;
use crate::judge::show;
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Default)]
pub struct Release {
    pub attachments: BTreeSet<String>,
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
            refusal: None,
        },
        Err(refusal) => Release {
            attachments: BTreeSet::new(),
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
    let mut found = BTreeSet::new();
    for (present, name) in [
        (!spec.binaries.is_empty(), "binary"),
        (spec.skill, "skill"),
        (spec.deb.is_some(), "deb"),
        (spec.cargo.is_some(), "cargo"),
        (spec.oci.is_some(), "oci"),
        (spec.chart.is_some(), "chart"),
        (spec.npm.is_some(), "npm"),
    ] {
        if present {
            found.insert(name.to_string());
        }
    }
    found
}

fn carriers(attachment: &str) -> &'static [&'static str] {
    match attachment {
        "binary" | "skill" | "deb" | "npm" | "oci" | "chart" => &["release-binary"],
        "cargo" => &["release-binary", "release-cargo"],
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
    let mut found = Found::new();
    if held.rust && !held.ignore.lines().any(|line| line.trim() == "target/") {
        found.push(Seed::wrong(
            &rule::CARGO_TARGET_IGNORED,
            "Cargo.toml without target/ in .gitignore".to_string(),
        ));
    }
    spec(held.release.refusal.as_deref(), &mut found);
    deliverable(&held.release, &held.root, &mut found);
    if held.ships.contains("binary") {
        for lane in ["release-exact", "release-stable"] {
            if !held.lanes.contains(lane) {
                found.push(Seed::wrong(
                    &rule::RELEASE_LANE_PRESENT,
                    format!("binary release without a {lane} lane"),
                ));
            } else if !bound(held, lane) {
                found.push(Seed::wrong(
                    &rule::RELEASE_SOURCE_BOUND,
                    format!("{lane} exposes or forwards a second source binding"),
                ));
            }
        }
    }
    site(held, &mut found);
    found
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

fn bound(held: &Shape, lane: &str) -> bool {
    std::fs::read_to_string(
        held.root
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

fn site(held: &Shape, found: &mut Found) {
    if held.sites.is_empty() {
        return;
    }
    if !held.lanes.contains("deploy") {
        found.push(Seed::wrong(
            &rule::SITE_DEPLOY_LANE,
            format!(
                "{} declares a site without a deploy lane",
                show(&held.sites)
            ),
        ));
    }
}
