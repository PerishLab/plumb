use super::{Found, Shape};
use crate::judge::catalog::rules::structure as rule;
use crate::judge::finding::Seed;
use crate::judge::show;
use std::collections::BTreeSet;
use std::path::Path;

pub fn release(root: &Path) -> bool {
    std::fs::read_to_string(root.join("plumb.toml"))
        .ok()
        .and_then(|text| text.parse::<toml::Table>().ok())
        .is_some_and(|manifest| manifest.contains_key("release"))
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
            }
        }
    }
    let registries = held
        .ships
        .iter()
        .filter(|held| held.as_str() != "binary")
        .cloned()
        .collect::<BTreeSet<_>>();
    if !registries.is_empty() && !held.wrappers.contains("release") {
        found.push(Seed::wrong(
            &rule::REGISTRY_RELEASE_WRAPPER_PRESENT,
            format!("declares {} without a release wrapper", show(&registries)),
        ));
    }
    site(held, &mut found);
    found
}

fn site(held: &Shape, found: &mut Found) {
    if held.sites.is_empty() {
        return;
    }
    for (rule, want, seated) in [
        (
            &rule::SITE_SHIP_WRAPPER,
            "a ship wrapper",
            held.wrappers.contains("ship"),
        ),
        (
            &rule::SITE_DEPLOY_LANE,
            "a deploy lane",
            held.lanes.contains("deploy"),
        ),
    ] {
        if !seated {
            found.push(Seed::wrong(
                rule,
                format!("{} declares a site without {want}", show(&held.sites)),
            ));
        }
    }
}
