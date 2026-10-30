use super::model::Model;
use super::record::Finding;
use super::walk::Harvest;
use std::collections::BTreeMap;

pub fn settle(model: &Model, harvest: Harvest) -> Vec<Finding> {
    let mut carriers: BTreeMap<String, Vec<String>> = harvest
        .direct
        .iter()
        .map(|(item, evidence)| (item.clone(), vec![evidence.clone()]))
        .collect();
    let mut changed = true;
    while changed {
        changed = false;
        for edge in &harvest.edges {
            if carriers.contains_key(&edge.caller) {
                continue;
            }
            let Some(chain) = carriers.get(&edge.callee) else {
                continue;
            };
            let mut via = vec![edge.callee.clone()];
            via.extend(chain.iter().take(7).cloned());
            carriers.insert(edge.caller.clone(), via);
            changed = true;
        }
    }
    let mut keep = vec![true; harvest.findings.len()];
    let mut findings = harvest.findings;
    for edge in &harvest.edges {
        let Some(index) = edge.finding else {
            continue;
        };
        let foreign = model
            .functions
            .get(&edge.callee)
            .is_some_and(|function| function.owner != edge.owner);
        match carriers.get(&edge.callee) {
            Some(chain) if foreign => findings[index].via = chain.clone(),
            _ => keep[index] = false,
        }
    }
    let mut kept: Vec<Finding> = findings
        .into_iter()
        .zip(keep)
        .filter_map(|(finding, kept)| kept.then_some(finding))
        .collect();
    pair(&mut kept);
    kept.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.line.cmp(&right.line))
            .then(left.column.cmp(&right.column))
            .then(left.role.cmp(right.role))
    });
    kept
}

struct Release {
    path: String,
    item: String,
    receiver: Option<String>,
    line: usize,
}

impl Release {
    fn closes(&self, finding: &Finding) -> bool {
        let place = self.path == finding.path && self.item == finding.item;
        place && self.receiver == finding.domain.receiver && self.line > finding.line
    }
}

fn pair(findings: &mut [Finding]) {
    let releases: Vec<Release> = findings
        .iter()
        .filter(|finding| finding.lifecycle == Some("release"))
        .map(|finding| Release {
            path: finding.path.clone(),
            item: finding.item.clone(),
            receiver: finding.domain.receiver.clone(),
            line: finding.line,
        })
        .collect();
    for finding in findings.iter_mut() {
        let open = finding
            .hold
            .as_ref()
            .is_some_and(|hold| matches!(hold.form, "manual" | "handle") && hold.release.is_none());
        if !open {
            continue;
        }
        let line = releases
            .iter()
            .find(|release| release.closes(finding))
            .map(|release| release.line);
        if let Some(hold) = finding.hold.as_mut() {
            hold.release = line;
        }
    }
}
