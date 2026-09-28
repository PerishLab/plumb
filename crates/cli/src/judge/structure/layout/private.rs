use crate::catalog::rules::structure as law;
use crate::judge::finding::{Seed, wrong};
use plumb::snapshot::Snapshot;

pub fn judge(snapshot: &Snapshot) -> Vec<Seed> {
    if !snapshot
        .entries()
        .iter()
        .any(|entry| entry.path().starts_with(".forgejo/"))
    {
        return Vec::new();
    }
    vec![wrong(
        &law::KNOWN_WORKFLOW,
        ".forgejo is residue of the archived Forgejo forge; GitHub is the only forge and workflows live in wharf"
            .to_string(),
    )]
}
