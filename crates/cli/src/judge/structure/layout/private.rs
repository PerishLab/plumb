use crate::catalog::rules::structure as law;
use crate::judge::finding::{Seed, wrong};
use plumb::snapshot::Snapshot;

pub fn judge(snapshot: &Snapshot, repository: &str) -> Vec<Seed> {
    if repository == "plumb"
        || !snapshot
            .entries()
            .iter()
            .any(|entry| entry.path().starts_with(".forgejo/"))
    {
        return Vec::new();
    }
    vec![wrong(
        &law::KNOWN_WORKFLOW,
        ".forgejo belongs to Plumb; product repositories dispatch the canonical ship atom"
            .to_string(),
    )]
}
