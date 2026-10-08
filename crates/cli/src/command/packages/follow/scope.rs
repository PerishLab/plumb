use super::work;
use std::collections::BTreeSet;
use std::path::Path;

pub(super) fn check(root: &Path, base: &str, head: &str) -> Result<(), String> {
    if !work::git(root, &["status", "--porcelain", "--untracked-files=all"])?.is_empty() {
        return Err("Auto scope source has preserved local payload".into());
    }
    if work::git(root, &["rev-parse", "HEAD"])? == head {
        return inspect(root, base, head);
    }
    let tree = work::git(root, &["rev-parse", &format!("{head}^{{tree}}")])?;
    let index = crate::command::guard::precommit::tree::Index::new(root, &tree)?;
    work::git(&index.root, &["checkout", "--detach", "--quiet", head])?;
    inspect(&index.root, base, head)
}

fn inspect(root: &Path, base: &str, head: &str) -> Result<(), String> {
    let mut paths = BTreeSet::from(["Cargo.toml".to_string(), "package.json".to_string()]);
    for commit in [base, head] {
        let listed = work::git(root, &["ls-tree", "-r", "--name-only", "-z", commit])?;
        paths.extend(
            listed
                .split('\0')
                .filter(|path| plumb::packages::allowed(path))
                .map(str::to_string),
        );
    }
    let report = plumb::boundary::check(plumb::boundary::Request {
        root,
        base,
        head,
        write: &paths.into_iter().collect::<Vec<_>>(),
    })
    .map_err(|error| error.to_string())?;
    let mut outside = report.outside.into_iter().collect::<BTreeSet<_>>();
    outside.extend(
        report
            .changed
            .into_iter()
            .filter(|path| !plumb::packages::allowed(path)),
    );
    if outside.is_empty() {
        Ok(())
    } else {
        Err(format!("Auto candidate has forbidden paths: {outside:?}"))
    }
}
