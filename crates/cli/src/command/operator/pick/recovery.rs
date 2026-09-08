use super::{command, text};
use std::path::Path;

pub(super) struct Seat<'a>(pub(super) &'a Path);

impl Seat<'_> {
    pub(super) fn delivered(&self, head: &str, sources: &[String]) -> Result<bool, String> {
        let list = text(
            "inspect delivered pick",
            command(
                self.0,
                [
                    "rev-list",
                    "--first-parent",
                    &format!("--max-count={}", sources.len()),
                    head,
                ],
            )?,
        )?;
        let commits = list.lines().rev().collect::<Vec<_>>();
        if commits.len() != sources.len() {
            return Ok(false);
        }
        for (commit, source) in commits.iter().zip(sources) {
            let held = Pick {
                root: self.0,
                parent: "",
                commit,
                source,
            };
            if !held.sourced()? {
                return Ok(false);
            }
        }
        let base = text(
            "inspect delivered base",
            command(self.0, ["rev-parse", &format!("{}^", commits[0])])?,
        )?;
        self.verify(&base, head, sources)?;
        plumb::guard::commit(self.0, head)?;
        Ok(true)
    }

    pub(super) fn clean(&self) -> Result<(), String> {
        let root = self.0;
        if !text("inspect worktree", command(root, ["status", "--short"])?)?.is_empty() {
            return Err("pick requires a clean worktree".into());
        }
        for state in ["CHERRY_PICK_HEAD", "MERGE_HEAD", "REVERT_HEAD", "sequencer"] {
            let path = text(
                "inspect Git operation",
                command(root, ["rev-parse", "--git-path", state])?,
            )?;
            if root
                .join(path)
                .try_exists()
                .map_err(|error| error.to_string())?
            {
                return Err(format!("pick refuses an unfinished Git operation: {state}"));
            }
        }
        Ok(())
    }

    pub(super) fn unmarked(&self, name: &str) -> Result<(), String> {
        let root = self.0;
        let version = name
            .strip_prefix("release/")
            .ok_or("pick requires a release line")?;
        let marker = text(
            "inspect stable marker",
            command(
                root,
                [
                    "for-each-ref",
                    "--format=%(objectname)",
                    &format!("refs/tags/{version}"),
                ],
            )?,
        )?;
        if !marker.is_empty() {
            return Err(format!(
                "stable marker {version} already stands; pick cannot move its version line"
            ));
        }
        Ok(())
    }

    pub(super) fn unchanged(&self, name: &str, expected: &str) -> Result<(), String> {
        let root = self.0;
        plumb::forgejo::git::fetch(root)?;
        let current = text(
            "reinspect release head",
            command(root, ["rev-parse", &format!("origin/{name}")])?,
        )?;
        if current != expected {
            return Err(format!(
                "{name} moved from {expected} to {current}; retained the local pick without pushing"
            ));
        }
        self.clean()?;
        self.unmarked(name)
    }

    pub(super) fn verify(&self, base: &str, head: &str, sources: &[String]) -> Result<(), String> {
        let root = self.0;
        let range = format!("{base}..{head}");
        let listed = text(
            "inspect picked commits",
            command(root, ["rev-list", "--reverse", &range])?,
        )?;
        let commits = listed.lines().collect::<Vec<_>>();
        if commits.len() != sources.len() {
            return Err("local pick does not contain exactly the requested source commits".into());
        }
        let mut parent = base;
        for (commit, source) in commits.iter().zip(sources) {
            let held = Pick {
                root,
                parent,
                commit,
                source,
            };
            held.verify()?;
            parent = commit;
        }
        Ok(())
    }
}

struct Pick<'a> {
    root: &'a Path,
    parent: &'a str,
    commit: &'a str,
    source: &'a str,
}

impl Pick<'_> {
    fn verify(&self) -> Result<(), String> {
        let parents = text(
            "inspect picked ancestry",
            command(
                self.root,
                ["rev-list", "--parents", "--max-count=1", self.commit],
            )?,
        )?;
        if parents.split_whitespace().collect::<Vec<_>>() != [self.commit, self.parent] {
            return Err(
                "local pick is not a linear continuation of the current remote base".into(),
            );
        }
        if !self.sourced()? {
            return Err(format!(
                "local pick {} does not name requested source {}",
                self.commit, self.source
            ));
        }
        let parent = text(
            "inspect source ancestry",
            command(self.root, ["rev-parse", &format!("{}^", self.source)])?,
        )?;
        let replay = text(
            "replay requested pick",
            command(
                self.root,
                [
                    "merge-tree",
                    "--write-tree",
                    &format!("--merge-base={parent}"),
                    self.parent,
                    self.source,
                ],
            )?,
        )?;
        let tree = text(
            "inspect picked tree",
            command(
                self.root,
                ["rev-parse", &format!("{}^{{tree}}", self.commit)],
            )?,
        )?;
        if replay.lines().next() != Some(tree.as_str()) {
            return Err(format!(
                "local pick {} differs from the replayed source tree",
                self.commit
            ));
        }
        Ok(())
    }

    fn sourced(&self) -> Result<bool, String> {
        let body = text(
            "inspect picked provenance",
            command(self.root, ["show", "-s", "--format=%B", self.commit])?,
        )?;
        let provenance = body
            .lines()
            .rev()
            .find(|line| line.starts_with("(cherry picked from commit "));
        let expected = format!("(cherry picked from commit {})", self.source);
        Ok(provenance == Some(expected.as_str()))
    }
}
