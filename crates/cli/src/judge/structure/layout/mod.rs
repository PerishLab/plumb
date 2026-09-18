mod content;
mod face;
mod names;
mod private;
mod retired;
pub(crate) mod rule;

pub(crate) use face::{Faces, authority};

use crate::catalog::rules::structure as law;
use crate::judge::finding::{Seed, blind, unknown, wrong};
use crate::shape::layout::{Declared, Group, Held, Read, Seat, affirm};
use plumb::snapshot::Snapshot;
use std::collections::BTreeSet;

pub fn judge(read: &Read, rooted: bool) -> Vec<Seed> {
    let Some(evidence) = &read.evidence else {
        return Vec::new();
    };
    verdict(
        &read.held,
        Tree(&evidence.snapshot, &evidence.repository, &evidence.affirmed),
        rooted,
    )
}

fn verdict(held: &Held, tree: Tree<'_>, rooted: bool) -> Vec<Seed> {
    match held {
        Held::Wrong(error) => vec![blind(&law::KNOWN_DIRECTORY, error.clone())],
        Held::Stated(declared) => tree.judge(declared, rooted),
        _ => Vec::new(),
    }
}

struct Tree<'a>(&'a Snapshot, &'a str, &'a affirm::Held);

fn named(group: &Group) -> Vec<String> {
    group.names.clone()
}

impl Tree<'_> {
    fn judge(&self, declared: &Declared, rooted: bool) -> Vec<Seed> {
        let mut found = retired::judge(self.0, declared);
        found.extend(self.covered(declared));
        if !rooted {
            found.extend(private::judge(self.0, self.1));
        }
        found.extend(self.anchored(declared));
        found.extend(self.ruled(declared));
        found.extend(declared.overrides.iter().map(|item| {
            Seed::noted(
                &law::DEFAULT_OVERRIDDEN,
                format!("plumb.toml overrides the Depot default {item}"),
            )
        }));
        found
    }

    fn ruled(&self, declared: &Declared) -> Vec<Seed> {
        let mut found = Vec::new();
        for seat in declared.seats.iter().filter(|seat| !seat.retired) {
            for held in &seat.rule {
                found.extend(self.applied(seat, held));
                found.extend(self.affirmed(declared, held, &self.leaves(seat, held)));
            }
        }
        for group in declared.groups.iter().filter(|group| !group.retired) {
            for held in &group.rule {
                found.extend(content::read(self.0, group, held));
                found.extend(self.affirmed(declared, held, &named(group)));
            }
        }
        found
    }

    fn leaves(&self, seat: &Seat, held: &str) -> Vec<String> {
        let Ok(member) = rule::parse(held).and_then(|held| rule::member(&held)) else {
            return Vec::new();
        };
        let (Some(container), Some(leaf)) = (seat.container(), member.leaf) else {
            return Vec::new();
        };
        self.members(container)
            .into_iter()
            .map(|name| format!("{name}/{leaf}"))
            .collect()
    }

    fn affirmed(&self, declared: &Declared, held: &str, targets: &[String]) -> Vec<Seed> {
        let Ok(member) = rule::parse(held).and_then(|held| rule::member(&held)) else {
            return Vec::new();
        };
        if member.affirms.is_empty() || targets.is_empty() {
            return Vec::new();
        }
        let unread = member
            .affirms
            .iter()
            .filter(|name| !rule::face(name))
            .cloned()
            .collect::<Vec<_>>();
        if !unread.is_empty() {
            return vec![blind(
                &law::SEAT_AFFIRMED,
                format!(
                    "{held} names faces this Plumb cannot read: {}",
                    unread.join(", ")
                ),
            )];
        }
        let faces = Faces(self.0).taken(declared, &member.affirms);
        let authority = authority(&faces);
        let mut found = Vec::new();
        for target in targets {
            if self.2.authority(target) == Some(authority.as_str()) {
                continue;
            }
            found.push(wrong(
                &law::SEAT_AFFIRMED,
                format!(
                    "{target} was affirmed against a different {}; reread it and run plumb affirm; see: plumb cookbook affirmed",
                    member.affirms.join(", ")
                ),
            ));
        }
        found
    }

    fn applied(&self, seat: &Seat, held: &str) -> Vec<Seed> {
        let member = match rule::parse(held).and_then(|held| rule::member(&held)) {
            Ok(member) => member,
            Err(error) => return vec![blind(&law::SEAT_MEMBER, error)],
        };
        let Some(container) = seat.container() else {
            return Vec::new();
        };
        let members = self.members(container);
        let mut found = Vec::new();
        if let Some(count) = member.count
            && members.len() != count
        {
            found.push(wrong(
                &law::SEAT_MEMBER,
                format!(
                    "{container} holds {} members where {held} fixes {count}; see: plumb cookbook seat",
                    members.len()
                ),
            ));
        }
        found.extend(self.capped(&members, &member, held));
        found.extend(names::judge(&members, &member, held));
        let Some(holds) = member.holds.as_deref() else {
            return found;
        };
        if !rule::known(holds) {
            found.push(blind(
                &law::SEAT_MEMBER,
                format!("{held} names a shape called {holds} this Plumb cannot read"),
            ));
            return found;
        }
        for name in members {
            let leaf = name.rsplit('/').next().unwrap_or(&name);
            if !rule::named(leaf, holds, self.1) {
                found.push(wrong(
                    &law::SEAT_MEMBER,
                    format!("{name} is not named as {held} requires; see: plumb cookbook seat"),
                ));
            }
        }
        found
    }

    fn covered(&self, declared: &Declared) -> Vec<Seed> {
        let heads = declared
            .seats
            .iter()
            .map(Seat::head)
            .collect::<BTreeSet<_>>();
        let names = declared
            .groups
            .iter()
            .flat_map(|group| group.names.iter().map(String::as_str))
            .collect::<BTreeSet<_>>();
        let (dirs, files) = self.top();
        let mut found = Vec::new();
        for name in dirs {
            if !heads.contains(name) {
                found.push(unknown(
                    &law::KNOWN_DIRECTORY,
                    format!("directory {name} sits in no declared seat; see: plumb cookbook seat"),
                ));
            }
        }
        for name in files {
            if !names.contains(name) {
                found.push(unknown(
                    &law::KNOWN_FILE,
                    format!("file {name} sits in no declared seat; see: plumb cookbook seat"),
                ));
            }
        }
        found
    }

    fn anchored(&self, declared: &Declared) -> Vec<Seed> {
        let mut found = Vec::new();
        for seat in declared.seats.iter().filter(|seat| !seat.retired) {
            let Some(container) = seat.container() else {
                continue;
            };
            let Some(anchor) = seat.anchor.as_ref().filter(|held| !held.is_empty()) else {
                continue;
            };
            found.extend(self.bare(container, anchor));
        }
        found
    }

    fn bare(&self, container: &str, anchor: &[String]) -> Vec<Seed> {
        let mut found = Vec::new();
        for member in self.members(container) {
            let held = anchor
                .iter()
                .any(|leaf| self.carried(&format!("{member}/{leaf}")));
            if held {
                continue;
            }
            found.push(wrong(
                &law::SEAT_ANCHORED,
                format!(
                    "{member} carries none of {}; see: plumb cookbook seat",
                    anchor.join(", ")
                ),
            ));
        }
        found
    }

    fn capped(&self, members: &BTreeSet<String>, member: &rule::Member, held: &str) -> Vec<Seed> {
        let (Some(leaf), Some(bytes)) = (member.leaf.as_deref(), member.bytes) else {
            return Vec::new();
        };
        let mut found = Vec::new();
        for name in members {
            let path = format!("{name}/{leaf}");
            let Some(entry) = self.0.entries().iter().find(|entry| entry.path() == path) else {
                found.push(wrong(
                    &law::SEAT_MEMBER,
                    format!("{path} is not a tracked leaf"),
                ));
                continue;
            };
            let held = format!(
                "{path} carries {} bytes where {held} caps {bytes}; see: plumb cookbook wayfinder",
                entry.bytes().len()
            );
            if entry.bytes().len() > bytes {
                found.push(wrong(&law::SEAT_MEMBER, held));
            }
        }
        found
    }

    fn top(&self) -> (BTreeSet<&str>, BTreeSet<&str>) {
        let mut dirs = BTreeSet::new();
        let mut files = BTreeSet::new();
        for entry in self.0.entries() {
            match entry.path().split_once('/') {
                Some((head, _)) => {
                    dirs.insert(head);
                }
                None => {
                    files.insert(entry.path());
                }
            }
        }
        (dirs, files)
    }

    fn members(&self, container: &str) -> BTreeSet<String> {
        let mut found = BTreeSet::new();
        for entry in self.0.seat(container) {
            let rest = entry.path().trim_start_matches(container).trim_matches('/');
            if let Some((head, _)) = rest.split_once('/') {
                found.insert(format!("{container}/{head}"));
            }
        }
        found
    }

    fn carried(&self, path: &str) -> bool {
        self.0.entries().iter().any(|entry| entry.path() == path)
    }
}
