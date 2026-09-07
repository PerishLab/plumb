use super::rule::Member;
use crate::catalog::rules::structure as law;
use crate::judge::finding::{Seed, wrong};
use std::collections::BTreeSet;

pub(super) fn judge(members: &BTreeSet<String>, rule: &Member, held: &str) -> Vec<Seed> {
    members
        .iter()
        .filter_map(|path| {
            let name = path.rsplit('/').next().unwrap_or(path);
            let denied = rule.deny.iter().any(|value| value == name);
            let allowed = rule
                .allow
                .as_ref()
                .is_none_or(|names| names.iter().any(|value| value == name));
            if allowed && !denied {
                return None;
            }
            Some(wrong(
                &law::SEAT_MEMBER,
                format!("{path} is outside the member names admitted by {held}"),
            ))
        })
        .collect()
}
