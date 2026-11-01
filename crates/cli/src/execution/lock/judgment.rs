use super::reference::Reference;
use super::review::Declaration;
use super::{Report, Scope};
use plumb::config::Execution;
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Serialize)]
pub(crate) struct Judgment {
    schema: &'static str,
    consistent: bool,
    guard: bool,
    declaration: Declaration,
    reports: Vec<Report>,
    references: BTreeSet<String>,
    limits: [&'static str; 3],
}

pub(crate) fn check(root: &Path, execution: &Execution, scope: Scope) -> Result<Judgment, String> {
    let declaration = Declaration::read(root)?;
    let compiler = super::version(execution, &["rustc", "-vV"])?;
    if super::host(&compiler)? != declaration.target {
        return Err("declared lock target differs from the actual compilation host".into());
    }
    let mut reports = Vec::new();
    for mode in &declaration.features {
        let scope = Scope {
            all: mode.all(),
            ..scope.clone()
        };
        let report = super::resolve(root, execution, scope)?;
        let at = report
            .argv
            .iter()
            .position(|arg| arg == "--target")
            .ok_or("lock report has no target")?;
        if report.argv.get(at + 1) != Some(&declaration.target) {
            return Err("declared lock target differs from the actual compilation host".into());
        }
        reports.push(report);
    }
    let references = references(&reports)?;
    let reviewed = declaration
        .review
        .iter()
        .map(|review| review.reference.clone())
        .collect::<BTreeSet<_>>();
    if references != reviewed {
        let missing = references
            .difference(&reviewed)
            .cloned()
            .collect::<Vec<_>>();
        let stale = reviewed
            .difference(&references)
            .cloned()
            .collect::<Vec<_>>();
        return Err(format!(
            "lock disposition mismatch; missing={missing:?}; stale={stale:?}"
        ));
    }
    Ok(Judgment {
        schema: "plumb.lock-review/v1",
        consistent: true,
        guard: false,
        declaration,
        reports,
        references,
        limits: [
            "consistency of exact source-bound human dispositions, not authentication or a proof of role, hold or release safety",
            "only the declared host/feature selections and five std File methods; unexecuted configurations and other APIs/languages are not admitted",
            "lib/test duplicates may share a source reference; no individual diagnostic cfg attestation, Ectropy policy admission or Guard activation",
        ],
    })
}

fn references(reports: &[Report]) -> Result<BTreeSet<String>, String> {
    let mut references = BTreeSet::new();
    for finding in reports.iter().flat_map(|report| &report.messages.findings) {
        match &finding.reference {
            Some(Reference::Bound { digest, .. }) => {
                references.insert(digest.clone());
            }
            Some(Reference::Unknown { reason }) => {
                return Err(format!(
                    "lock disposition has an unbound compiler reference: {reason}"
                ));
            }
            None => return Err("lock disposition has no compiler reference identity".into()),
        }
    }
    Ok(references)
}

impl Judgment {
    pub(crate) fn render(&self, json: bool) -> Result<String, String> {
        if json {
            return serde_json::to_string_pretty(self).map_err(|error| error.to_string());
        }
        Ok(format!(
            "lock reviews consistent: {} exact references across {} declared combinations; human dispositions only; no Guard activation",
            self.references.len(),
            self.reports.len()
        ))
    }
}
