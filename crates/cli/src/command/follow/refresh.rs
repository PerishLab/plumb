use super::{State, work};
use std::path::Path;

pub(super) fn prepare(state: &State) -> Result<Option<String>, String> {
    let root = &state.worktree;
    clean(root)?;
    let head = work::git(root, &["rev-parse", "HEAD"])?;
    let base = work::git(root, &["rev-parse", "origin/main"])?;
    let common = work::git(root, &["merge-base", &head, &base])?;
    if common == base {
        return Ok(None);
    }
    let known = state.pushed.as_ref() == Some(&head)
        || state.candidate.as_ref() == Some(&head)
        || state.plan.as_ref().is_some_and(|plan| plan.source == head);
    if !known {
        return Err("Auto local head moved outside recorded recovery state".into());
    }
    scope::check(root, &common, &head)?;
    let tree = match merged(root, &head, &base)? {
        Some(tree) => tree,
        None => work::git(root, &["rev-parse", &format!("{base}^{{tree}}")])?,
    };
    let tree = tree.as_str();
    if !matches!(tree.len(), 40 | 64) || !tree.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("Auto base merge did not produce an exact clean tree".into());
    }
    let commit = work::git(
        root,
        &[
            "commit-tree",
            tree,
            "-p",
            &head,
            "-p",
            &base,
            "-m",
            &format!("Resume follow on current main\n\nRefs #{}.", state.issue),
        ],
    )?;
    scope::check(root, &base, &commit)?;
    Ok(Some(commit))
}

fn merged(root: &Path, head: &str, base: &str) -> Result<Option<String>, String> {
    let output = plumb::config::detached("git")
        .arg("-C")
        .arg(root)
        .args(["merge-tree", "--write-tree", head, base])
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|error| error.to_string())?;
    match output.status.code() {
        Some(0) => String::from_utf8(output.stdout)
            .map_err(|error| error.to_string())?
            .lines()
            .next()
            .map(|tree| Some(tree.to_string()))
            .ok_or_else(|| "Auto base merge has no tree".into()),
        Some(1) => Ok(None),
        _ => Err(format!(
            "Auto base merge failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )),
    }
}

pub(super) fn apply(root: &Path, commit: &str) -> Result<(), String> {
    clean(root)?;
    let parents = work::git(root, &["show", "-s", "--format=%P", commit])?;
    let held = parents.split_whitespace().collect::<Vec<_>>();
    if held.len() != 2
        || work::git(root, &["rev-parse", "HEAD"])? != held[0]
        || work::git(root, &["rev-parse", "origin/main"])? != held[1]
    {
        return Err("Auto source or main moved during base refresh; reread before retry".into());
    }
    work::git(root, &["merge", "--ff-only", "--no-edit", commit])?;
    if work::git(root, &["rev-parse", "HEAD"])? != commit {
        return Err("Auto base refresh head disagrees".into());
    }
    clean(root)
}

fn clean(root: &Path) -> Result<(), String> {
    if work::git(root, &["status", "--porcelain", "--untracked-files=all"])?.is_empty() {
        Ok(())
    } else {
        Err(
            "Auto worktree has preserved local payload; commit or recover it before resuming"
                .into(),
        )
    }
}

pub(super) mod scope {
    use super::super::work;
    use std::collections::BTreeSet;
    use std::path::Path;

    pub(in super::super) fn check(root: &Path, base: &str, head: &str) -> Result<(), String> {
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
        if outside.contains("plumb.toml") && migrated(root, base, head)? {
            outside.remove("plumb.toml");
        }
        if outside.is_empty() {
            Ok(())
        } else {
            Err(format!("Auto candidate has forbidden paths: {outside:?}"))
        }
    }

    pub(in super::super) fn migrated(root: &Path, base: &str, head: &str) -> Result<bool, String> {
        let before = work::git(root, &["show", &format!("{base}:plumb.toml")])?;
        let after = work::git(root, &["show", &format!("{head}:plumb.toml")])?;
        let expected = crate::shape::release::migration::rewrite(&format!("{before}\n"))?;
        Ok(expected.is_some_and(|expected| expected.trim() == after))
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::super::pull::{body, change};
    use super::super::work::git as workgit;

    fn repository(manifest: &str) -> (tempfile::TempDir, String) {
        let root = tempfile::tempdir().unwrap();
        workgit(root.path(), &["init", "-q", "-b", "main"]).unwrap();
        workgit(root.path(), &["config", "user.name", "Fixture"]).unwrap();
        workgit(
            root.path(),
            &["config", "user.email", "fixture@example.invalid"],
        )
        .unwrap();
        std::fs::write(root.path().join("plumb.toml"), manifest).unwrap();
        std::fs::write(root.path().join("Cargo.lock"), "version = 4\n").unwrap();
        workgit(root.path(), &["add", "."]).unwrap();
        workgit(root.path(), &["commit", "-q", "-m", "base"]).unwrap();
        let base = workgit(root.path(), &["rev-parse", "HEAD"]).unwrap();
        (root, base)
    }

    fn head(root: &std::path::Path, message: &str) -> String {
        workgit(root, &["add", "-A"]).unwrap();
        workgit(root, &["commit", "-q", "-m", message]).unwrap();
        workgit(root, &["rev-parse", "HEAD"]).unwrap()
    }

    fn describe(root: &std::path::Path, base: &str, head: &str) -> String {
        body(
            520,
            change(root, base, head).unwrap(),
            "## Delta\nNo first-party package moved.\n",
        )
    }

    mod candidate {
        use super::*;

        #[test]
        fn lockfile() {
            let (root, base) = repository("[identity]\nns = \"probe\"\n");
            std::fs::write(
                root.path().join("Cargo.lock"),
                "version = 4\n\n[[package]]\nname = \"probe\"\nversion = \"0.2.0\"\n",
            )
            .unwrap();
            let head = head(root.path(), "follow");
            let body = describe(root.path(), &base, &head);
            assert!(body.contains(
                "## Change\nRegistered follow operation; manifests and lockfiles only.\n"
            ));
            assert!(!body.contains("[identity] ns"));
            assert!(body.contains("Refs #520."));
            assert!(body.contains("## Delta\nNo first-party package moved.\n"));
        }

        #[test]
        fn migration() {
            let (root, base) = repository("[release]\nproduct = \"probe\"\n");
            let before = std::fs::read_to_string(root.path().join("plumb.toml")).unwrap();
            let migrated = crate::shape::release::migration::rewrite(&before)
                .unwrap()
                .unwrap();
            std::fs::write(root.path().join("plumb.toml"), migrated).unwrap();
            let head = head(root.path(), "follow");
            let body = describe(root.path(), &base, &head);
            assert!(body.contains(
                "## Change\nRegistered follow operation; manifests and lockfiles only, plus the mechanical move of plumb.toml [release] product to [identity] ns.\n"
            ));
        }

        #[test]
        fn edit() {
            let (root, base) = repository("[identity]\nns = \"probe\"\n");
            std::fs::write(
                root.path().join("plumb.toml"),
                "[identity]\nns = \"probe\"\n\n[workflow]\n",
            )
            .unwrap();
            let head = head(root.path(), "follow");
            assert_eq!(
                change(root.path(), &base, &head).unwrap(),
                "Registered follow operation; manifests and lockfiles only."
            );
        }
    }
}
