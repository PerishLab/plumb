use crate::{rules::RULES, shape};
use std::collections::BTreeSet;

pub struct Note {
    pub grade: &'static str,
    pub dim: &'static str,
    pub line: String,
}

type Found = Vec<(&'static str, String)>;

const CONCURRENCY: &str = "concurrency:\n  group: guard-${{ github.event.pull_request.number || github.ref }}\n  cancel-in-progress: true";
const CONTAINER: &str = "mirror.perish.lan/ci/deno";
const COMPONENTS: &str =
    "packages/components is reserved; reusable components belong to the design system";

pub fn judge(held: &shape::Shape) -> Vec<Note> {
    let mut notes = Vec::new();
    for (dim, found) in [
        ("env", held.env()),
        ("structure", held.structure()),
        ("deps", held.deps()),
        ("web", held.web.clone().unwrap_or_default()),
        ("dispatch", held.dispatch.clone().unwrap_or_default()),
    ] {
        for (grade, line) in found {
            notes.push(Note { grade, dim, line });
        }
    }
    notes
}

impl shape::Shape {
    fn env(&self) -> Found {
        let mut found = Found::new();
        if self
            .edition
            .as_ref()
            .is_some_and(|edition| edition != "2024")
        {
            found.push((
                "out of true",
                format!(
                    "edition is {}, the skeleton holds 2024",
                    self.edition.as_deref().unwrap_or("")
                ),
            ));
        }
        let pinned = format!("{CONTAINER}:");
        if self.lane.as_ref().is_some_and(|yml| yml.contains(&pinned)) {
            found.push((
                "out of true",
                "CI container pinned to a tag, the skeleton tracks latest".to_string(),
            ));
        }
        found
    }
    fn structure(&self) -> Found {
        let mut found = Found::new();
        if self.runseal {
            for name in &RULES.required {
                if !self.wrappers.contains(name) {
                    found.push(("out of true", format!("no {name} wrapper")));
                }
            }
            for name in &RULES.hooks {
                if !self.hooks.contains(name) {
                    found.push(("out of true", format!("no {name} hook")));
                }
            }
            if !self.laws {
                found.push(("out of true", "no ectropy.toml".to_string()));
            }
            if self.wrappers.contains("guard")
                && !(self.guard.contains("plumb") && self.guard.contains("doctor"))
            {
                found.push(("out of true", "guard does not run plumb doctor".to_string()));
            }
            if self.wrappers.contains("init") && !self.init.contains("\"plumb\"") {
                found.push(("out of true", "init does not require plumb".to_string()));
            }
            if self
                .lane
                .as_ref()
                .is_some_and(|yml| !yml.contains(CONCURRENCY))
            {
                found.push((
                    "out of true",
                    "guard lane without the concurrency block".to_string(),
                ));
            }
        }
        if let Some(name) = &self.mint {
            found.push((
                "out of true",
                format!("publishable package {name} at the root, not under packages/"),
            ));
        }
        for (dir, name) in &self.packages {
            let last = name.rsplit('/').next().unwrap_or(name);
            if last != dir {
                found.push((
                    "out of true",
                    format!(
                        "package {name} sits in packages/{dir}, the directory must match the name"
                    ),
                ));
            }
        }
        if self.root.join("packages/components").is_dir() {
            found.push(("out of true", COMPONENTS.to_string()));
        }
        if let Some(why) = &self.unread {
            found.push((
                "blind",
                format!(
                    "cannot read ectropy.toml: {}",
                    why.lines().next().unwrap_or("")
                ),
            ));
        } else {
            for line in &self.policy {
                found.push(("out of true", line.clone()));
            }
        }
        for name in &self.dirs {
            if !RULES.dirs.contains(name) && !self.actions.contains(name) {
                found.push((
                    "unknown shape",
                    format!("directory {name} has no shadow in the skeleton"),
                ));
            }
        }
        for path in &self.operator_tests {
            found.push((
                "out of true",
                format!("{path} is a .runseal test; tested logic belongs in sealkit"),
            ));
        }
        for name in &self.wrappers {
            if !RULES.wrappers.contains(name) {
                found.push((
                    "unknown shape",
                    format!("wrapper {name} has no shadow in the skeleton"),
                ));
            }
        }
        self.paired(&mut found);
        self.matched(&mut found);
        self.anchored(&mut found);
        for name in &self.lanes {
            if !RULES.lanes.contains(name) {
                found.push((
                    "unknown shape",
                    format!("workflow {name} has no shadow in the skeleton"),
                ));
            }
        }
        found
    }
    fn deps(&self) -> Found {
        let mut found = Found::new();
        if self.binary && !self.clap {
            found.push((
                "out of true",
                "ships a rust binary without clap".to_string(),
            ));
        }
        if self.binary && !self.substrate {
            found.push((
                "out of true",
                "ships a rust binary without plumb".to_string(),
            ));
        }
        for (name, held) in &RULES.retired {
            if self.deno.contains(name.as_str()) {
                found.push((
                    "out of true",
                    format!("depends on {name}, renamed to {held}"),
                ));
            }
        }
        for name in RULES.pinned(&self.deno) {
            found.push((
                "out of true",
                format!("self-built {name} is version-pinned, the skeleton tracks latest"),
            ));
        }
        for (seat, name) in &self.node {
            if RULES.blacklist.contains(name) {
                found.push((
                    "out of true",
                    format!("{seat} depends on blacklisted styling package {name}"),
                ));
            }
        }
        found
    }

    fn paired(&self, found: &mut Found) {
        if self.rust && !self.ignore.lines().any(|line| line.trim() == "target/") {
            found.push((
                "out of true",
                "Cargo.toml without target/ in .gitignore".to_string(),
            ));
        }
        if self.wrappers.contains("release") {
            for lane in ["release-beta", "release-stable"] {
                if !self.lanes.contains(lane) {
                    found.push((
                        "out of true",
                        format!("release wrapper without a {lane} lane"),
                    ));
                }
            }
        }
        if !self.ships.is_empty() && !self.wrappers.contains("release") {
            found.push((
                "out of true",
                format!("declares {} without a release wrapper", show(&self.ships)),
            ));
        }
    }

    fn matched(&self, found: &mut Found) {
        if self.inits && self.listed.is_empty() {
            found.push((
                "blind",
                "an init wrapper is present but its required paths could not be read".to_string(),
            ));
        }
        for name in self.listed.difference(&self.wrappers) {
            found.push((
                "out of true",
                format!("init requires wrapper {name} which does not exist"),
            ));
        }
        for path in &self.bounds {
            if !self.root.join(path).exists() {
                found.push((
                    "out of true",
                    format!("boundary names {path} which does not exist"),
                ));
            }
        }
    }

    fn anchored(&self, found: &mut Found) {
        if !self.rust {
            return;
        }
        let Some(repo) = &self.repo else {
            return;
        };
        let anchors: Vec<&String> = self
            .named
            .iter()
            .filter(|(_, name)| name == repo)
            .map(|(seat, _)| seat)
            .collect();
        if anchors.is_empty() {
            found.push((
                "out of true",
                format!("no crate is named {repo}, the anchor is missing"),
            ));
        }
        if anchors.len() > 1 {
            found.push((
                "out of true",
                format!(
                    "{} crates are named {repo}, the anchor must be alone",
                    anchors.len()
                ),
            ));
        }
        let mut seats: BTreeSet<&str> = anchors.iter().map(|seat| seat.as_str()).collect();
        for (seat, _) in self.named.iter().filter(|(_, name)| name == "api") {
            if self.entries.contains(seat) {
                seats.insert(seat);
            } else {
                found.push(("out of true", format!("api crate at {seat} is headless")));
            }
        }
        for seat in &self.derived {
            if !seats.contains(seat.as_str()) {
                found.push((
                    "out of true",
                    format!("Cascade derives in {seat}, outside the anchor"),
                ));
            }
        }
    }
}

pub fn show(set: &BTreeSet<String>) -> String {
    if set.is_empty() {
        return "-".to_string();
    }
    set.iter().cloned().collect::<Vec<_>>().join(" ")
}
