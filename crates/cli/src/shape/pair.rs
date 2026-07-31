use super::{Found, Shape};
use crate::judge::catalog::rules::structure as rule;
use crate::judge::finding::Seed;
use crate::judge::show;
use std::collections::BTreeSet;
use std::path::Path;

pub fn ships(root: &Path) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let Some(held) = declared(root) else {
        return found;
    };
    if held.contains_key("binaries") {
        found.insert("binary".to_string());
    }
    if held.contains_key("cargo") {
        found.insert("cargo".to_string());
    }
    found
}

fn declared(root: &Path) -> Option<toml::Table> {
    std::fs::read_to_string(root.join("plumb.toml"))
        .ok()
        .and_then(|text| text.parse::<toml::Table>().ok())
        .and_then(|manifest| manifest.get("release").cloned())
        .and_then(|held| held.as_table().cloned())
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
