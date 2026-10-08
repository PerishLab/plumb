use serde::Serialize;
use std::path::{Path, PathBuf};

use super::tree;

const CODE: &str = "guard.integration-branch";

#[derive(Serialize)]
pub struct Finding {
    schema: &'static str,
    root: PathBuf,
    ok: bool,
    code: &'static str,
    message: String,
    remedy: String,
}

impl Finding {
    pub fn render(&self, json: bool) {
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(self).expect("Guard finding should encode")
            );
        } else {
            eprintln!("plumb guard: {}: {}", self.code, self.message);
            eprintln!("see: {}", self.remedy);
        }
    }
}

pub fn inspect(root: &Path) -> Result<(), Finding> {
    let branch = tree::git(
        root,
        &["branch", "--show-current"],
        "read the current branch",
    )
    .unwrap_or_default();
    match branch.as_str() {
        "main" | "master" => Err(Finding {
            schema: "plumb.guard-finding/v1",
            root: root.to_path_buf(),
            ok: false,
            code: CODE,
            message: format!(
                "{branch} is an integration branch; begin mutable work from an Issue-managed topic worktree"
            ),
            remedy: format!("plumb cookbook {CODE}"),
        }),
        _ => Ok(()),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Snapshot {
    pub branch: String,
    pub head: String,
    tree: String,
}

struct Outcome {
    source: String,
    head: String,
    proof: plumb::guard::Descriptor,
}

#[derive(Serialize)]
struct Evidence<'a> {
    schema: &'static str,
    root: &'a Path,
    source: &'a str,
    head: &'a str,
    tree: &'a str,
    guard: &'a plumb::guard::Descriptor,
}

#[derive(Clone, Copy)]
struct Refresh<'a>(&'a Path);

pub(crate) fn renew(root: &Path) -> Result<plumb::guard::Descriptor, String> {
    Refresh(root).execute().map(|outcome| outcome.proof)
}

pub(super) fn refresh(root: &Path, json: bool) -> i32 {
    Refresh(root).run(json)
}

impl Refresh<'_> {
    fn run(self, json: bool) -> i32 {
        if let Err(finding) = inspect(self.0) {
            finding.render(json);
            return 1;
        }
        match self.execute() {
            Ok(outcome) => self.report(&outcome, json),
            Err(error) => {
                eprintln!("plumb guard: {error}");
                1
            }
        }
    }

    fn execute(self) -> Result<Outcome, String> {
        let before = Snapshot::read(self.0)?;
        let proof = super::action::prove(self.0)?;
        let current = Snapshot::read(self.0)?;
        if current != before {
            return Err(format!(
                "HEAD or worktree changed while Guard ran: {} became {}",
                before.head, current.head
            ));
        }
        if proof.tree != before.tree {
            return Err(format!(
                "Guard proved tree {}, not HEAD tree {}",
                proof.tree, before.tree
            ));
        }
        let head = tree::bind(self.0, &before, &proof)?;
        let after = Snapshot::read(self.0)?;
        if after.branch != before.branch || after.head != head || after.tree != before.tree {
            return Err("refreshed HEAD does not retain the exact branch and tree".into());
        }
        if plumb::guard::commit(self.0, &head)? != proof {
            return Err("refreshed HEAD carries a different Guard proof".into());
        }
        Ok(Outcome {
            source: before.head,
            head,
            proof,
        })
    }

    fn report(self, outcome: &Outcome, json: bool) -> i32 {
        let evidence = Evidence {
            schema: "plumb.guard-refresh/v1",
            root: self.0,
            source: &outcome.source,
            head: &outcome.head,
            tree: &outcome.proof.tree,
            guard: &outcome.proof,
        };
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&evidence).expect("Guard refresh should encode")
            );
        } else {
            println!("plumb guard refresh {}", self.0.display());
            println!();
            println!("  source  {}", outcome.source);
            println!("  head    {}", outcome.head);
            println!("  tree    {}", outcome.proof.tree);
            println!("  proof   {}", outcome.proof.digest);
            println!();
            println!("  refreshed HEAD carries the current Guard proof");
        }
        0
    }
}

impl Snapshot {
    fn read(root: &Path) -> Result<Self, String> {
        let status = tree::git(
            root,
            &["status", "--porcelain=v1", "--untracked-files=all"],
            "read Guard refresh status",
        )?;
        if !status.is_empty() {
            return Err("Guard proof refresh refuses a dirty worktree or index".into());
        }
        let branch = tree::git(
            root,
            &["symbolic-ref", "--quiet", "HEAD"],
            "read attached Guard refresh branch",
        )
        .map_err(|_| {
            "Guard proof refresh refuses detached or ambiguous HEAD; use an attached topic branch"
                .to_string()
        })?;
        Ok(Self {
            branch,
            head: tree::git(
                root,
                &["rev-parse", "--verify", "HEAD^{commit}"],
                "read Guard refresh HEAD",
            )?,
            tree: tree::git(
                root,
                &["rev-parse", "--verify", "HEAD^{tree}"],
                "read Guard refresh tree",
            )?,
        })
    }
}
