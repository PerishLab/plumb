use crate::shape::lane;
use crate::shape::layout::Declared;
use plumb::depot::sha;
use plumb::snapshot::Snapshot;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct Faces<'a>(pub &'a Snapshot);

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Standing<'a> {
    Current,
    Transitional(Option<&'a str>),
    Stale,
}

pub(crate) struct Authority<'a> {
    current: String,
    priors: Vec<(String, Option<&'a str>)>,
}

impl<'a> Authority<'a> {
    pub(crate) fn judge(&self, recorded: Option<&str>) -> Standing<'a> {
        let Some(recorded) = recorded else {
            return Standing::Stale;
        };
        if self.current == recorded {
            return Standing::Current;
        }
        for (held, note) in &self.priors {
            if held == recorded {
                return Standing::Transitional(*note);
            }
        }
        Standing::Stale
    }
}

impl Faces<'_> {
    pub(crate) fn evaluate<'a>(&self, declared: &'a Declared, held: &[String]) -> Authority<'a> {
        Surface(declared).evaluate(&self.lanes(), held)
    }

    pub(crate) fn taken(&self, declared: &Declared, held: &[String]) -> BTreeMap<String, String> {
        Surface(declared).taken(&self.lanes(), held)
    }

    pub(crate) fn members(&self, container: &str) -> BTreeSet<String> {
        let mut found = BTreeSet::new();
        for entry in self.0.seat(container) {
            let rest = entry.path().trim_start_matches(container).trim_matches('/');
            if let Some((head, _)) = rest.split_once('/') {
                found.insert(format!("{container}/{head}"));
            }
        }
        found
    }

    fn lanes(&self) -> String {
        let mut held = self
            .0
            .entries()
            .iter()
            .filter_map(|entry| lane::name(entry.path()))
            .map(str::to_string)
            .collect::<Vec<_>>();
        held.sort();
        held.join("\n")
    }
}

pub(crate) fn authority(faces: &BTreeMap<String, String>) -> String {
    let mut body = String::new();
    for (name, held) in faces {
        body.push_str(name);
        body.push(' ');
        body.push_str(held);
        body.push('\n');
    }
    sha(body.as_bytes())
}

struct Surface<'a>(&'a Declared);

impl<'a> Surface<'a> {
    fn evaluate(&self, lanes: &str, held: &[String]) -> Authority<'a> {
        let current = authority(&self.taken(lanes, held));
        let priors = self
            .0
            .transitions
            .iter()
            .map(|prior| {
                (
                    authority(&Surface(&prior.declared).taken(lanes, held)),
                    prior.note.as_deref(),
                )
            })
            .collect();
        Authority { current, priors }
    }

    fn taken(&self, lanes: &str, held: &[String]) -> BTreeMap<String, String> {
        let mut found = BTreeMap::new();
        for name in held {
            let body = match name.as_str() {
                "declaration" => self.declaration(),
                "seat" => self.seats(),
                "lane" => lanes.to_string(),
                _ => continue,
            };
            found.insert(name.clone(), sha(body.as_bytes()));
        }
        found
    }

    fn seats(&self) -> String {
        let mut held = self
            .0
            .seats
            .iter()
            .filter(|seat| !seat.retired)
            .map(|seat| seat.path.clone())
            .collect::<Vec<_>>();
        held.extend(
            self.0
                .groups
                .iter()
                .filter(|group| !group.retired)
                .flat_map(|group| group.names.clone()),
        );
        held.sort();
        held.join("\n")
    }

    fn declaration(&self) -> String {
        let sorted = |held: &[String]| {
            let mut held = held.to_vec();
            held.sort();
            held.join(",")
        };
        let mut held = self
            .0
            .seats
            .iter()
            .map(|seat| {
                format!(
                    "seat {} anchor={} rule={} retired={}",
                    seat.path,
                    seat.anchor.as_deref().map_or("-".to_string(), sorted),
                    sorted(&seat.rule),
                    seat.retired,
                )
            })
            .collect::<Vec<_>>();
        held.extend(self.0.groups.iter().map(|group| {
            format!(
                "file {} rule={} retired={}",
                sorted(&group.names),
                sorted(&group.rule),
                group.retired,
            )
        }));
        held.sort();
        held.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::{Standing, Surface, authority};
    use crate::shape::layout::{Declared, Group, Seat, Transition};

    fn faces() -> Vec<String> {
        ["declaration", "seat", "lane"].map(str::to_string).to_vec()
    }

    fn base(groups: Vec<Group>, extra: Option<&str>) -> Declared {
        let mut seats = vec![Seat {
            path: "crates/*".into(),
            anchor: Some(vec!["Cargo.toml".into()]),
            rule: Vec::new(),
            scan: Vec::new(),
            retired: false,
        }];
        if let Some(path) = extra {
            seats.push(Seat {
                path: path.into(),
                anchor: None,
                rule: Vec::new(),
                scan: Vec::new(),
                retired: false,
            });
        }
        Declared {
            seats,
            groups,
            ..Declared::default()
        }
    }

    fn group(names: &[&str], retired: bool) -> Group {
        Group {
            names: names.iter().map(|name| (*name).to_string()).collect(),
            rule: Vec::new(),
            retired,
        }
    }

    fn prior(declared: Declared, note: Option<&str>) -> Transition {
        Transition {
            declared,
            note: note.map(str::to_string),
        }
    }

    #[test]
    fn transition() {
        let held = faces();
        let old = ["plumb.toml", "ectropy.toml", "runseal.toml"];
        let before = base(vec![group(&old, false)], None);
        let recorded = authority(&Surface(&before).taken("crates", &held));
        let next = vec![
            group(&["plumb.toml", "runseal.toml"], false),
            group(&["ectropy.toml"], true),
        ];
        let mut after = base(next.clone(), None);
        after
            .transitions
            .push(prior(before, Some("retire ectropy.toml")));

        let judged = Surface(&after).evaluate("crates", &held);
        assert_eq!(
            judged.judge(Some(&recorded)),
            Standing::Transitional(Some("retire ectropy.toml"))
        );
        assert_eq!(judged.judge(Some(&judged.current)), Standing::Current);
        assert_eq!(judged.judge(None), Standing::Stale);
        assert_eq!(
            Surface(&after)
                .evaluate("apps\ncrates", &held)
                .judge(Some(&recorded)),
            Standing::Stale
        );

        let mut drifted = base(next, Some("docs"));
        drifted.transitions.push(prior(
            base(vec![group(&old, false)], Some("docs")),
            Some("retire ectropy.toml"),
        ));
        assert_eq!(
            Surface(&drifted)
                .evaluate("crates", &held)
                .judge(Some(&recorded)),
            Standing::Stale
        );

        after.transitions.clear();
        assert_eq!(
            Surface(&after)
                .evaluate("crates", &held)
                .judge(Some(&recorded)),
            Standing::Stale
        );
    }

    #[test]
    fn staged() {
        let held = faces();
        let old = ["plumb.toml", "ectropy.toml", "runseal.toml"];
        let first = base(vec![group(&old, false)], None);
        let second = base(
            vec![
                group(&["plumb.toml", "runseal.toml"], false),
                group(&["ectropy.toml"], true),
            ],
            None,
        );
        let mut third = base(
            vec![
                group(&["plumb.toml"], false),
                group(&["ectropy.toml", "runseal.toml"], true),
            ],
            None,
        );
        third
            .transitions
            .push(prior(first.clone(), Some("stage-1")));
        third.transitions.push(prior(second.clone(), None));

        let judged = Surface(&third).evaluate("crates", &held);
        let one = authority(&Surface(&first).taken("crates", &held));
        let two = authority(&Surface(&second).taken("crates", &held));
        assert_eq!(
            judged.judge(Some(&one)),
            Standing::Transitional(Some("stage-1"))
        );
        assert_eq!(judged.judge(Some(&two)), Standing::Transitional(None));
        assert_eq!(judged.judge(Some(&judged.current)), Standing::Current);
    }
}
