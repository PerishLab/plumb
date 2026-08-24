use crate::judge::finding;
use crate::shape;
use shape::Found;

pub fn judge(evidence: &shape::lane::Evidence) -> Found {
    evidence
        .projected()
        .iter()
        .filter(|lane| lane.drifted())
        .map(|lane| {
            let state = if lane.absent() {
                "is absent"
            } else {
                "was hand-edited or rendered by an older Plumb"
            };
            let evidence = format!("lane {} {state}; run plumb lane --write", lane.path);
            finding::Seed::noted(&crate::catalog::rules::release::LANE_RENDERED, evidence)
        })
        .collect()
}
