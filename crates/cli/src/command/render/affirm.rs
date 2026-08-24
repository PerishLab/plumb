use crate::judge::structure::layout::{Faces, authority, rule};
use crate::shape::layout::{Declared, Held};
use plumb::snapshot::Snapshot;
use std::path::Path;

pub const SEAT: &str = ".plumb/affirmed.toml";

pub struct Owed {
    pub target: String,
    pub rule: String,
    pub authority: String,
}

pub fn owed(root: &Path) -> Result<Vec<Owed>, String> {
    let snapshot = Snapshot::read(root).map_err(|error| error.to_string())?;
    let Held::Stated(declared) = crate::shape::layout::stated(root) else {
        return Err("no layout is declared".to_string());
    };
    let mut found = Vec::new();
    for seat in declared.seats.iter().filter(|seat| !seat.retired) {
        for held in &seat.rule {
            let Ok(member) = rule::parse(held).and_then(|held| rule::member(&held)) else {
                continue;
            };
            let (Some(container), Some(leaf)) = (seat.container(), member.leaf.as_deref()) else {
                continue;
            };
            let targets = Faces(&snapshot)
                .members(container)
                .into_iter()
                .map(|name| format!("{name}/{leaf}"))
                .collect::<Vec<_>>();
            found.extend(Stamp(&snapshot, &declared, &member).owed(held, &targets));
        }
    }
    for group in declared.groups.iter().filter(|group| !group.retired) {
        for held in &group.rule {
            let Ok(member) = rule::parse(held).and_then(|held| rule::member(&held)) else {
                continue;
            };
            found.extend(Stamp(&snapshot, &declared, &member).owed(held, &group.names));
        }
    }
    Ok(found)
}

struct Stamp<'a>(&'a Snapshot, &'a Declared, &'a rule::Member);

impl Stamp<'_> {
    fn owed(&self, rule: &str, targets: &[String]) -> Vec<Owed> {
        if self.2.affirms.is_empty() {
            return Vec::new();
        }
        let authority = authority(&Faces(self.0).taken(self.1, &self.2.affirms));
        targets
            .iter()
            .map(|target| Owed {
                target: target.clone(),
                rule: rule.to_string(),
                authority: authority.clone(),
            })
            .collect()
    }
}

pub fn write(root: &Path, owed: &[Owed]) -> Result<(), String> {
    let mut body = String::new();
    for held in owed {
        body.push_str(&record(&held.target, &held.rule, &held.authority));
        body.push('\n');
    }
    let seat = root.join(SEAT);
    if let Some(base) = seat.parent() {
        std::fs::create_dir_all(base).map_err(|error| error.to_string())?;
    }
    std::fs::write(&seat, body).map_err(|error| error.to_string())
}

fn record(target: &str, rule: &str, authority: &str) -> String {
    format!("[[record]]\ntarget = \"{target}\"\nrule = \"{rule}\"\nauthority = \"{authority}\"\n")
}
