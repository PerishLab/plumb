use super::Owed;
use crate::judge::structure::layout::{Faces, authority, rule};
use crate::shape::layout::{Declared, Held};
use crate::shape::product::Profile;
use plumb::snapshot::Snapshot;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Receipt {
    schema: String,
    profile: String,
    records: Vec<Record>,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Record {
    document: Owed,
    content: String,
    definition: String,
}

struct Review<'a> {
    snapshot: &'a Snapshot,
    profile: &'a Profile,
    declared: &'a Declared,
    bodies: &'a BTreeMap<String, Vec<u8>>,
}

impl Receipt {
    pub fn read(
        root: &Path,
        profile: &Profile,
        bodies: &BTreeMap<String, Vec<u8>>,
    ) -> Result<Self, String> {
        let Held::Stated(declared) = crate::shape::layout::parse(&profile.manifest) else {
            return Err("candidate review requires a declared layout".into());
        };
        let snapshot = Snapshot::read(root).map_err(|error| error.to_string())?;
        let review = Review {
            snapshot: &snapshot,
            profile,
            declared: &declared,
            bodies,
        };
        let records = review.records()?;
        Ok(Self {
            schema: "plumb.affirmation/v1".into(),
            profile: profile.digest.clone(),
            records,
        })
    }

    pub fn encode(&self) -> Result<Vec<u8>, String> {
        toml::to_string(self)
            .map(String::into_bytes)
            .map_err(|error| format!("cannot encode candidate affirmation: {error}"))
    }

    pub fn prefix(&self) -> String {
        Self::namespace(&self.profile)
    }

    pub fn namespace(profile: &str) -> String {
        format!("profiles/affirmed/{profile}/")
    }

    pub fn path(&self, bytes: &[u8]) -> String {
        format!("{}{}.toml", self.prefix(), plumb::depot::sha(bytes))
    }

    pub fn owed(&self) -> Vec<Owed> {
        self.records
            .iter()
            .map(|record| record.document.clone())
            .collect()
    }

    pub fn confirmed(&self, bodies: &BTreeMap<String, Vec<u8>>) -> Result<bool, String> {
        let mut found = false;
        for (path, bytes) in bodies
            .iter()
            .filter(|(path, _)| path.starts_with(&self.prefix()))
        {
            let text = std::str::from_utf8(bytes)
                .map_err(|error| format!("cannot read {path}: {error}"))?;
            let held: Self = toml::from_str(text)
                .map_err(|error| format!("invalid candidate affirmation {path}: {error}"))?;
            if held.schema != self.schema
                || held.profile != self.profile
                || self.path(bytes) != *path
            {
                return Err(format!("candidate affirmation binding drift: {path}"));
            }
            found |= held == *self;
        }
        Ok(found)
    }
}

impl Review<'_> {
    fn records(&self) -> Result<Vec<Record>, String> {
        let mut found = Vec::new();
        for seat in self.declared.seats.iter().filter(|seat| !seat.retired) {
            for reference in &seat.rule {
                let (member, digest) = self.member(reference)?;
                let (Some(container), Some(leaf)) = (seat.container(), member.leaf.as_deref())
                else {
                    continue;
                };
                let targets = Faces(self.snapshot)
                    .members(container)
                    .into_iter()
                    .map(|name| format!("{name}/{leaf}"))
                    .collect::<Vec<_>>();
                found.extend(self.targeted(reference, (&member, &digest), &targets)?);
            }
        }
        for group in self.declared.groups.iter().filter(|group| !group.retired) {
            for reference in &group.rule {
                let (member, digest) = self.member(reference)?;
                found.extend(self.targeted(reference, (&member, &digest), &group.names)?);
            }
        }
        found.sort_by(|left, right| {
            (&left.document.target, &left.document.rule)
                .cmp(&(&right.document.target, &right.document.rule))
        });
        found.dedup();
        Ok(found)
    }

    fn member(&self, reference: &str) -> Result<(rule::Member, String), String> {
        let reference = rule::parse(reference)?;
        let path = format!("rules/{}.toml", reference.set);
        let bytes = self
            .bodies
            .get(&path)
            .ok_or_else(|| format!("candidate lacks {path}"))?;
        let text = std::str::from_utf8(bytes).map_err(|error| error.to_string())?;
        let doc = text
            .parse::<toml::Table>()
            .map_err(|error| error.to_string())?;
        let definition = serde_json::to_vec(rule::definition(&reference, &doc)?)
            .map_err(|error| error.to_string())?;
        Ok((
            rule::decode(&reference, &doc)?,
            plumb::depot::sha(&definition),
        ))
    }

    fn targeted(
        &self,
        reference: &str,
        rule: (&rule::Member, &str),
        targets: &[String],
    ) -> Result<Vec<Record>, String> {
        let (member, definition) = rule;
        if member.affirms.is_empty() {
            return Ok(Vec::new());
        }
        if member.affirms.iter().any(|name| !rule::face(name)) {
            return Err(format!("{reference} names an unread affirmation face"));
        }
        let mut faces = Faces(self.snapshot).taken(self.declared, &member.affirms);
        if faces.contains_key("declaration") {
            faces.insert(
                "declaration".into(),
                plumb::depot::sha(self.profile.manifest.as_bytes()),
            );
        }
        let authority = authority(&faces);
        targets
            .iter()
            .map(|target| {
                let entry = self
                    .snapshot
                    .entries()
                    .iter()
                    .find(|entry| entry.path() == target)
                    .ok_or_else(|| format!("review requires tracked document {target}"))?;
                regular(self.snapshot.root(), target)?;
                if !matches!(entry.mode(), "100644" | "100755") {
                    return Err(format!("review requires regular document {target}"));
                }
                Ok(Record {
                    document: Owed {
                        target: target.clone(),
                        rule: reference.into(),
                        authority: authority.clone(),
                    },
                    content: plumb::depot::sha(entry.bytes()),
                    definition: definition.into(),
                })
            })
            .collect()
    }
}

fn regular(root: &Path, target: &str) -> Result<(), String> {
    let mut path = root.to_path_buf();
    for part in Path::new(target).components() {
        let std::path::Component::Normal(part) = part else {
            return Err(format!("review requires anchored document {target}"));
        };
        path.push(part);
        let held = std::fs::symlink_metadata(&path)
            .map_err(|error| format!("cannot inspect reviewed document {target}: {error}"))?;
        if held.is_symlink() {
            return Err(format!("review refuses symbolic document path {target}"));
        }
    }
    if !path.is_file() {
        return Err(format!("review requires regular document {target}"));
    }
    Ok(())
}
