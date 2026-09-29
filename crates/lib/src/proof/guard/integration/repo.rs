use super::{Checkout, Expectation, Inspection, Refusal, Relation, inventory, process};
use std::path::{Path, PathBuf};

pub struct Repository {
    root: PathBuf,
}

impl Repository {
    pub fn open(path: &Path) -> Result<Self, Refusal> {
        let root = std::fs::canonicalize(path).map_err(|error| {
            Refusal::new(
                "integration.root",
                format!("cannot resolve {}: {error}", path.display()),
            )
        })?;
        if !root.is_dir() {
            return Err(Refusal::new(
                "integration.root",
                format!("{} is not a directory", root.display()),
            ));
        }
        let repository = Self { root };
        let top = repository.required(
            &["rev-parse", "--show-toplevel"],
            "integration.worktree",
            "path is not a Git worktree",
        )?;
        let top = std::fs::canonicalize(top).map_err(|error| {
            Refusal::new(
                "integration.worktree",
                format!("cannot resolve worktree: {error}"),
            )
        })?;
        if top != repository.root {
            return Err(Refusal::new(
                "integration.root",
                format!(
                    "expected worktree root at {}, found {}",
                    repository.root.display(),
                    top.display()
                ),
            ));
        }
        Ok(repository)
    }

    pub fn inspect(&self, expected: &Expectation) -> Result<Inspection, Refusal> {
        let target = self.exact(&expected.target)?;
        let tree = self.required(
            &["rev-parse", "--verify", &format!("{target}^{{tree}}")],
            "integration.target",
            "cannot resolve target tree",
        )?;
        let head = self.required(
            &["rev-parse", "--verify", "HEAD"],
            "integration.head",
            "cannot resolve HEAD",
        )?;
        let headtree = self.required(
            &["rev-parse", "--verify", "HEAD^{tree}"],
            "integration.head",
            "cannot resolve HEAD tree",
        )?;
        let branch = self.optional(&["symbolic-ref", "--quiet", "--short", "HEAD"])?;
        let upstream = self.upstream(branch.as_deref())?;
        let tracked = self.optional(&[
            "rev-parse",
            "--verify",
            &format!("{}^{{commit}}", expected.tracking),
        ])?;
        let clean = process::success(
            self.git(&["status", "--porcelain=v1", "-z", "--untracked-files=all"])?,
            "integration.status",
            "cannot inspect integration checkout",
        )?
        .is_empty();
        let common = self.required(
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
            "integration.git",
            "cannot resolve the Git common directory",
        )?;
        let common = std::fs::canonicalize(common).map_err(|error| {
            Refusal::new(
                "integration.git",
                format!("cannot resolve the Git common directory: {error}"),
            )
        })?;
        Ok(Inspection {
            expected: Expectation {
                target,
                ..expected.clone()
            },
            tree,
            checkout: Checkout {
                path: self.root.clone(),
                common,
                branch,
                head: head.clone(),
                tree: headtree,
                clean,
                upstream,
                tracked,
            },
            relation: self.relation(&head, &expected.target)?,
            worktrees: inventory::list(&self.root)?,
        })
    }

    pub fn advance(&self, expected: &Expectation, observed: &str) -> Result<Inspection, Refusal> {
        let before = self.inspect(expected)?;
        self.validate(&before, observed)?;
        match before.relation {
            Relation::Equal => return Ok(before),
            Relation::Ahead => {
                return Err(Refusal::observed(
                    "integration.ahead",
                    "integration checkout is ahead of the exact target",
                    before,
                ));
            }
            Relation::Diverged => {
                return Err(Refusal::observed(
                    "integration.diverged",
                    "integration checkout has diverged from the exact target",
                    before,
                ));
            }
            Relation::Behind => {}
        }
        self.required(
            &["merge", "--ff-only", "--no-edit", &before.expected.target],
            "integration.advance",
            "cannot fast-forward integration checkout",
        )?;
        let after = self.inspect(expected)?;
        self.verify(&after)?;
        Ok(after)
    }

    fn validate(&self, held: &Inspection, observed: &str) -> Result<(), Refusal> {
        let refuse = |code, message| Err(Refusal::observed(code, message, held.clone()));
        if held.checkout.head != observed {
            return refuse(
                "integration.changed",
                format!(
                    "HEAD changed from observed {observed} to {}",
                    held.checkout.head
                ),
            );
        }
        if held.checkout.branch.as_deref() != Some(held.expected.branch.as_str()) {
            return refuse(
                "integration.branch",
                format!(
                    "checkout is not on declared branch {}",
                    held.expected.branch
                ),
            );
        }
        if !held.checkout.clean {
            return refuse(
                "integration.dirty",
                "integration checkout is not clean".to_string(),
            );
        }
        if held.checkout.upstream.as_deref() != Some(held.expected.tracking.as_str()) {
            return refuse(
                "integration.upstream",
                format!("branch does not track {}", held.expected.tracking),
            );
        }
        if held.checkout.tracked.as_deref() != Some(held.expected.target.as_str()) {
            return refuse(
                "integration.tracking",
                format!(
                    "{} is absent or does not equal the exact target",
                    held.expected.tracking
                ),
            );
        }
        Ok(())
    }

    fn verify(&self, held: &Inspection) -> Result<(), Refusal> {
        let failed = || {
            Refusal::observed(
                "integration.verify",
                "integration checkout did not retain the exact post-advance state",
                held.clone(),
            )
        };
        if held.relation != Relation::Equal || held.checkout.head != held.expected.target {
            return Err(failed());
        }
        if held.checkout.tree != held.tree || !held.checkout.clean {
            return Err(failed());
        }
        if held.checkout.branch.as_deref() != Some(held.expected.branch.as_str()) {
            return Err(failed());
        }
        if held.checkout.upstream.as_deref() != Some(held.expected.tracking.as_str()) {
            return Err(failed());
        }
        Ok(())
    }

    fn exact(&self, target: &str) -> Result<String, Refusal> {
        let resolved = self.required(
            &["rev-parse", "--verify", &format!("{target}^{{commit}}")],
            "integration.target",
            "exact target is not an already-fetched commit",
        )?;
        if resolved != target {
            return Err(Refusal::new(
                "integration.target",
                format!("target must be the exact commit {resolved}"),
            ));
        }
        Ok(resolved)
    }

    fn relation(&self, head: &str, target: &str) -> Result<Relation, Refusal> {
        if head == target {
            return Ok(Relation::Equal);
        }
        if self.ancestor(head, target)? {
            return Ok(Relation::Behind);
        }
        if self.ancestor(target, head)? {
            return Ok(Relation::Ahead);
        }
        Ok(Relation::Diverged)
    }

    fn ancestor(&self, older: &str, newer: &str) -> Result<bool, Refusal> {
        let output = self.git(&["merge-base", "--is-ancestor", older, newer])?;
        match output.status.code() {
            Some(0) => Ok(true),
            Some(1) => Ok(false),
            _ => Err(process::failed(
                output,
                "integration.relation",
                "cannot compare commits",
            )),
        }
    }

    fn upstream(&self, branch: Option<&str>) -> Result<Option<String>, Refusal> {
        let Some(branch) = branch else {
            return Ok(None);
        };
        let reference = format!("refs/heads/{branch}");
        self.optional(&["for-each-ref", "--format=%(upstream:short)", &reference])
    }

    fn optional(&self, args: &[&str]) -> Result<Option<String>, Refusal> {
        let output = self.git(args)?;
        if output.status.success() {
            let held = process::text(output.stdout);
            Ok((!held.is_empty()).then_some(held))
        } else {
            Ok(None)
        }
    }

    fn required(
        &self,
        args: &[&str],
        code: &'static str,
        message: &str,
    ) -> Result<String, Refusal> {
        Ok(process::text(process::success(
            self.git(args)?,
            code,
            message,
        )?))
    }

    fn git(&self, args: &[&str]) -> Result<std::process::Output, Refusal> {
        process::run(&self.root, args)
    }
}
