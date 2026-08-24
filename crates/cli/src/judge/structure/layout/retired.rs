use crate::catalog::rules::structure as law;
use crate::judge::finding::{Seed, wrong};
use crate::shape::layout::Declared;
use plumb::snapshot::Snapshot;

pub(super) fn judge(snapshot: &Snapshot, declared: &Declared) -> Vec<Seed> {
    let mut found = Vec::new();
    for seat in declared.seats.iter().filter(|seat| seat.retired) {
        let root = seat.container().unwrap_or(&seat.path).trim_matches('/');
        let prefix = format!("{root}/");
        if snapshot
            .entries()
            .iter()
            .any(|entry| entry.path() == root || entry.path().starts_with(&prefix))
        {
            found.push(wrong(
                &law::RETIRED_SEAT_ABSENT,
                format!("retired seat {} still holds tracked paths", seat.path),
            ));
        }
    }
    for group in declared.groups.iter().filter(|group| group.retired) {
        for name in &group.names {
            if snapshot.entries().iter().any(|entry| entry.path() == name) {
                found.push(wrong(
                    &law::RETIRED_SEAT_ABSENT,
                    format!("retired root file {name} is still tracked"),
                ));
            }
        }
    }
    found
}
