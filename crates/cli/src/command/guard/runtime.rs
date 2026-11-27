use plumb::guard::Descriptor;
use serde::Serialize;
use std::path::{Path, PathBuf};

const SCHEMA: &str = plumb::guard::Runtime::SCHEMA;
const STRENGTH: &str = "PLUMB_GUARD_STRENGTH";
const BOUNDARY: &str = "PLUMB_GUARD_BOUNDARY";
pub(super) const ROLLOVER: &str = "guard.control.rollover";

pub(super) struct Selection;

#[derive(Debug, Eq, PartialEq)]
pub(super) enum Fault {
    Control(String),
    Other(String),
}

impl From<String> for Fault {
    fn from(message: String) -> Self {
        Self::Other(message)
    }
}

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

impl Evidence {
    fn resolved(&self) {
        if let Some(resolution) = &self.guard.resolution {
            println!("  context  {}", resolution.context);
            println!("  resolved {}", resolution.tree);
            for package in &resolution.packages {
                println!(
                    "  package  {} {} {}",
                    package.ecosystem, package.name, package.version
                );
            }
        }
    }
}

#[derive(Serialize)]
struct Refusal<'a> {
    schema: &'static str,
    root: &'a Path,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<&'static str>,
    message: &'a str,
}

impl Refusal<'_> {
    fn emit(&self, json: bool) -> i32 {
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(self).expect("runtime Guard refusal should encode")
            );
        } else {
            eprintln!("plumb guard: {}", self.message);
        }
        1
    }
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
                    evidence.resolved();
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
            Err(Fault::Control(error)) => Refusal {
                schema: SCHEMA,
                root,
                ok: false,
                code: Some(ROLLOVER),
                message: &error,
            }
            .emit(json),
            Err(Fault::Other(error)) => refused(root, &error, json),
        }
    }
}

pub(super) fn refused(root: &Path, message: &str, json: bool) -> i32 {
    Refusal {
        schema: SCHEMA,
        root,
        ok: false,
        code: None,
        message,
    }
    .emit(json)
}

#[locus::trace(with = crate::observation::view())]
fn prove(root: &Path) -> Result<Evidence, Fault> {
    let before = Head::read(root)?;
    crate::diagnostic::event(
        "source",
        || serde_json::json!({"commit":before.commit,"tree":before.tree,"strength":"full","boundary":"head"}),
    );
    before.clean(root)?;
    let guard = super::precommit::prove(root)?;
    let after = Head::read(root)?;
    after.clean(root)?;
    if before != after {
        return Err(Fault::Other(format!(
            "HEAD changed while Guard ran: {} became {}",
            before.commit, after.commit
        )));
    }
    if guard.tree != after.tree {
        return Err(Fault::Other(format!(
            "Guard proved tree {}, not HEAD tree {}",
            guard.tree, after.tree
        )));
    }
    let digest = plumb::guard::Runtime::seal(&after.commit, &guard)?;
    Ok(Evidence {
        schema: SCHEMA,
        root: root.to_path_buf(),
        ok: true,
        strength: "full",
        boundary: "head",
        commit: after.commit,
        guard,
        digest,
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

pub(super) fn changed(
    root: &Path,
    channels: Option<&plumb::packages::Channels>,
) -> Result<Option<String>, String> {
    let Some(channels) = channels else {
        return Ok(None);
    };
    let changes = crate::diagnostic::measure("packages.reobserve", || channels.changes(root))
        .map_err(|error| format!("guard.packages.unavailable: {error}"))?;
    if changes.is_empty() {
        return Ok(None);
    }
    crate::diagnostic::event(
        "packages.rollover",
        || serde_json::json!({"changes":changes}),
    );
    Ok(Some(format!(
        "guard.packages.rollover: {}",
        changes.join(", ")
    )))
}

#[cfg(test)]
mod tests {
    use super::{ROLLOVER, Refusal, SCHEMA};
    use std::path::Path;

    #[test]
    fn refusal() {
        let plain = serde_json::to_value(Refusal {
            schema: SCHEMA,
            root: Path::new("/repo"),
            ok: false,
            code: None,
            message: "stale at start",
        })
        .expect("plain");
        assert_eq!(plain["ok"], false);
        assert!(plain.get("code").is_none());

        let control = serde_json::to_value(Refusal {
            schema: SCHEMA,
            root: Path::new("/repo"),
            ok: false,
            code: Some(ROLLOVER),
            message: "guard.control.rollover: running Plumb v0.166.0 rolled over to stable v0.167.0 during Guard",
        })
        .expect("control");
        assert_eq!(control["ok"], false);
        assert_eq!(control["code"], "guard.control.rollover");
    }
}
