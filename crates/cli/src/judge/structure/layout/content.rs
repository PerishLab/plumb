use super::rule::Member;
use crate::catalog::rules::structure as law;
use crate::judge::finding::{Seed, blind, wrong};
use crate::shape::layout::Group;
use plumb::snapshot::Snapshot;

pub(super) fn read(snapshot: &Snapshot, group: &Group, held: &str) -> Vec<Seed> {
    match super::rule::parse(held).and_then(|held| super::rule::member(&held)) {
        Ok(member) => judge(snapshot, group, &member),
        Err(error) => vec![blind(&law::SEAT_MEMBER, error)],
    }
}

pub(super) fn judge(snapshot: &Snapshot, group: &Group, member: &Member) -> Vec<Seed> {
    group
        .names
        .iter()
        .flat_map(|path| {
            let content = Content(snapshot);
            let mut found = content.structured(path, &member.fields);
            if let Some(rule) = &member.lines {
                found.extend(content.file(path, rule));
            }
            found
        })
        .collect()
}

struct Content<'a>(&'a Snapshot);

impl Content<'_> {
    fn structured(&self, path: &str, rules: &[plumb::rule::Fields]) -> Vec<Seed> {
        if rules.is_empty() {
            return Vec::new();
        }
        let Some(entry) = self.0.entries().iter().find(|entry| entry.path() == path) else {
            return vec![wrong(
                &law::SEAT_MEMBER,
                format!("{path} is missing its governed content"),
            )];
        };
        let document = match plumb::rule::document(path, entry.bytes()) {
            Ok(document) => document,
            Err(error) => return vec![blind(&law::SEAT_MEMBER, format!("{path}: {error}"))],
        };
        rules
            .iter()
            .flat_map(|rule| match rule.check(&document) {
                Ok(issues) => issues
                    .into_iter()
                    .map(|issue| wrong(&law::SEAT_MEMBER, format!("{path}: {issue}")))
                    .collect(),
                Err(error) => vec![blind(&law::SEAT_MEMBER, format!("{path}: {error}"))],
            })
            .collect()
    }

    fn file(&self, path: &str, rule: &super::rule::Lines) -> Vec<Seed> {
        let Some(entry) = self.0.entries().iter().find(|entry| entry.path() == path) else {
            return vec![wrong(
                &law::SEAT_MEMBER,
                format!("{path} is missing its governed content"),
            )];
        };
        let text = match std::str::from_utf8(entry.bytes()) {
            Ok(text) => text,
            Err(error) => {
                return vec![blind(
                    &law::SEAT_MEMBER,
                    format!("{path} content is not UTF-8: {error}"),
                )];
            }
        };
        let lines = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect::<Vec<_>>();
        let mut found = Vec::new();
        for required in &rule.required {
            if !lines.contains(&required.as_str()) {
                found.push(wrong(
                    &law::SEAT_MEMBER,
                    format!("{path} is missing required content: {required}"),
                ));
            }
        }
        for line in lines {
            if rule.deny.iter().any(|value| value == line)
                || rule
                    .allow
                    .as_ref()
                    .is_some_and(|names| !names.iter().any(|value| value == line))
            {
                found.push(wrong(
                    &law::SEAT_MEMBER,
                    format!("{path} carries unapproved content: {line}"),
                ));
            }
        }
        found
    }
}
