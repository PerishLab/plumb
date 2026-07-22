use crate::rules::RULES;
use crate::shape;
use std::collections::BTreeSet;

pub struct Note {
    pub grade: &'static str,
    pub dim: &'static str,
    pub line: String,
}

type Found = Vec<(&'static str, String)>;

const GUARD_CONCURRENCY: &str = "concurrency:\n  group: guard-${{ github.event.pull_request.number || github.ref }}\n  cancel-in-progress: true";

const CI_CONTAINER: &str = "mirror.perish.lan/ci/deno";

pub fn judge(held: &shape::Shape) -> Vec<Note> {
    let mut notes = Vec::new();
    for (dim, found) in [
        ("env", env(held)),
        ("structure", structure(held)),
        ("deps", deps(held)),
    ] {
        for (grade, line) in found {
            notes.push(Note { grade, dim, line });
        }
    }
    notes
}

fn env(held: &shape::Shape) -> Found {
    let mut found = Found::new();
    if held
        .edition
        .as_ref()
        .is_some_and(|edition| edition != "2024")
    {
        found.push((
            "out of true",
            format!(
                "edition is {}, the skeleton holds 2024",
                held.edition.as_deref().unwrap_or("")
            ),
        ));
    }
    let pinned = format!("{CI_CONTAINER}:");
    if held
        .guard_lane
        .as_ref()
        .is_some_and(|yml| yml.contains(&pinned))
    {
        found.push((
            "out of true",
            "CI container pinned to a tag, the skeleton tracks latest".to_string(),
        ));
    }
    found
}

fn structure(held: &shape::Shape) -> Found {
    let mut found = Found::new();
    if held.runseal {
        for name in &RULES.required {
            if !held.wrappers.contains(name) {
                found.push(("out of true", format!("no {name} wrapper")));
            }
        }
        if !held.laws {
            found.push(("out of true", "no negentropy.toml".to_string()));
        }
        if held
            .guard_lane
            .as_ref()
            .is_some_and(|yml| !yml.contains(GUARD_CONCURRENCY))
        {
            found.push((
                "out of true",
                "guard lane without the concurrency block".to_string(),
            ));
        }
    }
    if let Some(name) = &held.root_package {
        found.push((
            "out of true",
            format!("publishable package {name} at the root, not under packages/"),
        ));
    }
    for (dir, name) in &held.packages {
        let last = name.rsplit('/').next().unwrap_or(name);
        if last != dir {
            found.push((
                "out of true",
                format!("package {name} sits in packages/{dir}, the directory must match the name"),
            ));
        }
    }
    if let Some(why) = &held.unread {
        found.push((
            "blind",
            format!(
                "cannot read negentropy.toml: {}",
                why.lines().next().unwrap_or("")
            ),
        ));
    } else {
        for (key, seen, want) in [("block", held.block, 4), ("path", held.path, 4)] {
            if seen.is_some_and(|value| value != want) {
                found.push((
                    "out of true",
                    format!(
                        "limit {key} is {}, the skeleton holds {want}",
                        seen.unwrap_or(0)
                    ),
                ));
            }
        }
        if held.laws && !held.grants.contains("test") {
            found.push(("out of true", "no test grant".to_string()));
        }
    }
    for name in &held.dirs {
        if !RULES.dirs.contains(name) {
            found.push((
                "unknown shape",
                format!("directory {name} has no shadow in the skeleton"),
            ));
        }
    }
    for name in &held.wrappers {
        if !RULES.wrappers.contains(name) {
            found.push((
                "unknown shape",
                format!("wrapper {name} has no shadow in the skeleton"),
            ));
        }
    }
    paired(held, &mut found);
    matched(held, &mut found);
    for name in &held.lanes {
        if !RULES.lanes.contains(name) {
            found.push((
                "unknown shape",
                format!("workflow {name} has no shadow in the skeleton"),
            ));
        }
    }
    found
}

fn deps(held: &shape::Shape) -> Found {
    let mut found = Found::new();
    if held.binary && !held.clap {
        found.push((
            "out of true",
            "ships a rust binary without clap".to_string(),
        ));
    }
    for (name, held_use) in &RULES.retired {
        if held.deno.contains(name.as_str()) {
            found.push((
                "out of true",
                format!("depends on {name}, renamed to {held_use}"),
            ));
        }
    }
    for name in pinned(&held.deno) {
        found.push((
            "out of true",
            format!("self-built {name} is version-pinned, the skeleton tracks latest"),
        ));
    }
    found
}

fn paired(held: &shape::Shape, found: &mut Found) {
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
}

fn matched(held: &shape::Shape, found: &mut Found) {
    if held.inits && held.listed.is_empty() {
        found.push((
            "blind",
            "an init wrapper is present but its required paths could not be read".to_string(),
        ));
    }
    for name in held.listed.difference(&held.wrappers) {
        found.push((
            "out of true",
            format!("init requires wrapper {name} which does not exist"),
        ));
    }
    for path in &held.bounds {
        if !held.root.join(path).exists() {
            found.push((
                "out of true",
                format!("boundary names {path} which does not exist"),
            ));
        }
    }
}

fn pinned(deno: &str) -> BTreeSet<String> {
    let marker = format!("jsr:{}/", RULES.scope);
    let mut found = BTreeSet::new();
    for chunk in deno.split(marker.as_str()).skip(1) {
        let name: String = chunk
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        if chunk[name.len()..].starts_with('@') {
            found.insert(format!("{}/{name}", RULES.scope));
        }
    }
    found
}

pub fn show(set: &BTreeSet<String>) -> String {
    if set.is_empty() {
        return "-".to_string();
    }
    set.iter().cloned().collect::<Vec<_>>().join(" ")
}
