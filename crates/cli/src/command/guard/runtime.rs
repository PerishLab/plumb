use plumb::guard::{Action, Descriptor};
use serde::Serialize;
use sha2::{Digest as _, Sha256};
use std::path::{Path, PathBuf};

const SCHEMA: &str = "plumb.guard-runtime/v1";
const STRENGTH: &str = "PLUMB_GUARD_STRENGTH";
const BOUNDARY: &str = "PLUMB_GUARD_BOUNDARY";

pub(super) struct Selection;

#[derive(Serialize)]
struct Evidence {
    schema: &'static str,
    root: PathBuf,
    ok: bool,
    strength: &'static str,
    boundary: &'static str,
    commit: String,
    guard: Descriptor,
    digest: String,
}

#[derive(Serialize)]
struct Claim<'a> {
    schema: &'static str,
    strength: &'static str,
    boundary: &'static str,
    repository: &'a str,
    commit: &'a str,
    tree: &'a str,
    plumb: &'a str,
    depot: &'a str,
    platform: &'a str,
    actions: &'a [Action],
    proof: &'a str,
}

#[derive(Serialize)]
struct Refusal<'a> {
    schema: &'static str,
    root: &'a Path,
    ok: bool,
    message: &'a str,
}

impl Selection {
    pub(super) fn read() -> Result<Option<Self>, String> {
        let strength = value(STRENGTH)?;
        let boundary = value(BOUNDARY)?;
        match (strength.as_deref(), boundary.as_deref()) {
            (None, None) => Ok(None),
            (Some("full"), Some("head")) => Ok(Some(Self)),
            (None, Some(_)) | (Some(_), None) => Err(format!(
                "{STRENGTH} and {BOUNDARY} must be declared together"
            )),
            (Some(strength), Some("head")) => {
                Err(format!("{STRENGTH} must be full, not {strength:?}"))
            }
            (Some("full"), Some(boundary)) => {
                Err(format!("{BOUNDARY} must be head, not {boundary:?}"))
            }
            (Some(strength), Some(boundary)) => Err(format!(
                "{STRENGTH} must be full and {BOUNDARY} must be head, not {strength:?}/{boundary:?}"
            )),
        }
    }

    pub(super) fn run(self, root: &Path, json: bool) -> i32 {
        match prove(root) {
            Ok(evidence) => {
                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&evidence)
                            .expect("runtime Guard evidence should encode")
                    );
                } else {
                    println!("plumb guard {}", evidence.root.display());
                    println!();
                    println!("  strength full");
                    println!("  boundary head");
                    println!("  commit   {}", evidence.commit);
                    println!("  tree     {}", evidence.guard.tree);
                    println!("  plumb    {}", evidence.guard.plumb);
                    println!("  proof    {}", evidence.guard.digest);
                    println!("  evidence {}", evidence.digest);
                    println!(
                        "  actions  {}",
                        evidence
                            .guard
                            .actions
                            .iter()
                            .map(|action| action.name.as_str())
                            .collect::<Vec<_>>()
                            .join(" ")
                    );
                    println!();
                    println!("  exact clean HEAD proved");
                }
                0
            }
            Err(error) => refused(root, &error, json),
        }
    }
}

pub(super) fn refused(root: &Path, message: &str, json: bool) -> i32 {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&Refusal {
                schema: SCHEMA,
                root,
                ok: false,
                message,
            })
            .expect("runtime Guard refusal should encode")
        );
    } else {
        eprintln!("plumb guard: {message}");
    }
    1
}

fn prove(root: &Path) -> Result<Evidence, String> {
    let before = Head::read(root)?;
    before.clean(root)?;
    let guard = super::precommit::prove(root)?;
    let after = Head::read(root)?;
    after.clean(root)?;
    if before != after {
        return Err(format!(
            "HEAD changed while Guard ran: {} became {}",
            before.commit, after.commit
        ));
    }
    if guard.tree != after.tree {
        return Err(format!(
            "Guard proved tree {}, not HEAD tree {}",
            guard.tree, after.tree
        ));
    }
    let claim = Claim {
        schema: SCHEMA,
        strength: "full",
        boundary: "head",
        repository: &guard.repository,
        commit: &after.commit,
        tree: &guard.tree,
        plumb: &guard.plumb,
        depot: &guard.depot,
        platform: &guard.platform,
        actions: &guard.actions,
        proof: &guard.digest,
    };
    let bytes = serde_json::to_vec(&claim)
        .map_err(|error| format!("cannot seal runtime Guard evidence: {error}"))?;
    Ok(Evidence {
        schema: SCHEMA,
        root: root.to_path_buf(),
        ok: true,
        strength: "full",
        boundary: "head",
        commit: after.commit,
        guard,
        digest: format!("{:x}", Sha256::digest(bytes)),
    })
}

#[derive(Eq, PartialEq)]
struct Head {
    commit: String,
    tree: String,
}

impl Head {
    fn read(root: &Path) -> Result<Self, String> {
        Ok(Self {
            commit: super::precommit::tree::git(
                root,
                &["rev-parse", "--verify", "HEAD"],
                "read HEAD",
            )?,
            tree: super::precommit::tree::git(
                root,
                &["rev-parse", "--verify", "HEAD^{tree}"],
                "read HEAD tree",
            )?,
        })
    }

    fn clean(&self, root: &Path) -> Result<(), String> {
        let status = super::precommit::tree::git(
            root,
            &["status", "--porcelain=v1", "--untracked-files=all"],
            "inspect the HEAD worktree",
        )?;
        if !status.is_empty() {
            return Err("PLUMB_GUARD_BOUNDARY=head requires an exact clean HEAD worktree".into());
        }
        let staged = plumb::guard::tree(root)?;
        if staged != self.tree {
            return Err(format!(
                "staged tree {staged} disagrees with HEAD tree {}",
                self.tree
            ));
        }
        Ok(())
    }
}

fn value(name: &str) -> Result<Option<String>, String> {
    std::env::var_os(name)
        .map(|value| {
            value
                .into_string()
                .map_err(|_| format!("{name} is not UTF-8"))
        })
        .transpose()
}
