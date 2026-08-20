use super::{Declared, Held, Seat, rule};
use crate::judge::catalog::rules::structure as law;
use crate::judge::finding::{Seed, blind, unknown, wrong};
use plumb::snapshot::Snapshot;
use std::collections::BTreeSet;

pub fn judge(snapshot: &Snapshot, held: &Held, repository: &str) -> Vec<Seed> {
    match held {
        Held::Wrong(error) => vec![blind(&law::KNOWN_DIRECTORY, error.clone())],
        Held::Stated(declared) => Tree(snapshot, repository).judge(declared),
        _ => Vec::new(),
    }
}

struct Tree<'a>(&'a Snapshot, &'a str);

impl Tree<'_> {
    fn judge(&self, declared: &Declared) -> Vec<Seed> {
        let mut found = self.covered(declared);
        found.extend(self.anchored(declared));
        found.extend(self.ruled(declared));
        found
    }

    fn ruled(&self, declared: &Declared) -> Vec<Seed> {
        let mut found = Vec::new();
        for seat in declared.seats.iter().filter(|seat| !seat.retired) {
            for held in &seat.rule {
                found.extend(self.applied(seat, held));
            }
        }
        for group in declared.groups.iter().filter(|group| !group.retired) {
            for held in &group.rule {
                if let Err(error) = rule::parse(held).and_then(|held| rule::member(&held)) {
                    found.push(blind(&law::SEAT_MEMBER, error));
                }
            }
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
                    "{container} holds {} members where {held} fixes {count}",
                    members.len()
                ),
            ));
        }
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
                    format!("{name} is not named as {held} requires"),
                ));
            }
        }
        found
    }

    fn covered(&self, declared: &Declared) -> Vec<Seed> {
        let heads = declared
            .seats
            .iter()
            .filter(|seat| !seat.retired)
            .map(Seat::head)
            .collect::<BTreeSet<_>>();
        let names = declared
            .groups
            .iter()
            .filter(|group| !group.retired)
            .flat_map(|group| group.names.iter().map(String::as_str))
            .collect::<BTreeSet<_>>();
        let (dirs, files) = self.top();
        let mut found = Vec::new();
        for name in dirs {
            if !heads.contains(name) {
                found.push(unknown(
                    &law::KNOWN_DIRECTORY,
                    format!("directory {name} sits in no declared seat"),
                ));
            }
        }
        for name in files {
            if !names.contains(name) {
                found.push(unknown(
                    &law::KNOWN_FILE,
                    format!("file {name} sits in no declared seat"),
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
                format!("{member} carries none of {}", anchor.join(", ")),
            ));
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
