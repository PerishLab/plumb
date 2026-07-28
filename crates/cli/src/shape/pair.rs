use super::{Found, Shape};
use crate::judge::show;
use std::collections::BTreeSet;
use std::path::Path;

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
        found.push((
            "out of true",
            "Cargo.toml without target/ in .gitignore".to_string(),
        ));
    }
    if held.wrappers.contains("release") {
        for lane in ["release-beta", "release-stable"] {
            if !held.lanes.contains(lane) {
                found.push((
                    "out of true",
                    format!("release wrapper without a {lane} lane"),
                ));
            }
        }
    }
    if !held.ships.is_empty() && !held.wrappers.contains("release") {
        found.push((
            "out of true",
            format!("declares {} without a release wrapper", show(&held.ships)),
        ));
    }
    site(held, &mut found);
    found
}

fn site(held: &Shape, found: &mut Found) {
    if held.sites.is_empty() {
        return;
    }
    for (want, seated) in [
        ("a ship wrapper", held.wrappers.contains("ship")),
        ("a deploy lane", held.lanes.contains("deploy")),
    ] {
        if !seated {
            found.push((
                "out of true",
                format!("{} declares a site without {want}", show(&held.sites)),
            ));
        }
    }
}
