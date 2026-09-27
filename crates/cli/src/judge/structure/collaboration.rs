use crate::catalog::rules::structure as law;
use crate::judge::finding::{Seed, wrong};
use crate::shape::layout::Read;

const POLICY: &str = "https://github.com/PerishLab/.github/blob/main/GOVERNANCE.md";

pub fn judge(read: &Read) -> Vec<Seed> {
    let Some(evidence) = &read.evidence else {
        return Vec::new();
    };
    if evidence.repository == ".github" {
        return Vec::new();
    }
    evidence
        .snapshot
        .entries()
        .iter()
        .filter_map(|entry| {
            let path = entry.path();
            reserved(path).then(|| {
                wrong(
                    &law::COLLABORATION_SURFACE_INHERITED,
                    format!(
                        "{path} is a repository-local collaboration template; inherit the organization surface instead; see: {POLICY}"
                    ),
                )
            })
        })
        .collect()
}

fn reserved(path: &str) -> bool {
    let files = [
        ".github/PULL_REQUEST_TEMPLATE",
        ".github/PULL_REQUEST_TEMPLATE.md",
    ];
    let trees = [".github/ISSUE_TEMPLATE/", ".github/PULL_REQUEST_TEMPLATE/"];
    files.contains(&path) || trees.iter().any(|tree| path.starts_with(tree))
}
