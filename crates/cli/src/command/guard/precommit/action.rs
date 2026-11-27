pub(super) use super::super::world::Binding;
use super::cargo::Lease;
use super::tree::{self, Index};
use super::world::{Check, Prepared};
use super::{Selection, cache, cargo};
use plumb::guard::Descriptor;
use std::path::Path;

pub(super) fn prove(root: &Path) -> Result<Descriptor, String> {
    prepare(root, false)
}

pub(super) fn resolved(root: &Path) -> Result<Descriptor, String> {
    prepare(root, true)
}

#[locus::trace(with = crate::observation::view())]
fn prepare(root: &Path, latest: bool) -> Result<Descriptor, String> {
    let _held = crate::execution::signal::Hold::new()?;
    crate::diagnostic::measure("guard.prove", || Proof { root, latest }.run())
}

struct Proof<'a> {
    root: &'a Path,
    latest: bool,
}

enum Refusal {
    Rollover(String),
    Check(String),
}

impl From<String> for Refusal {
    fn from(error: String) -> Self {
        Self::Check(error)
    }
}

impl Proof<'_> {
    fn run(&self) -> Result<Descriptor, String> {
        let tree = plumb::guard::tree(self.root)?;
        crate::diagnostic::event("staged", || serde_json::json!({"tree":tree}));
        let seat = cargo::seat(self.root, &tree)?;
        if let Some(proof) = self.local(&tree, seat.as_deref())? {
            return Ok(proof);
        }
        let lease = seat.is_some().then(|| Lease::new(self.root)).transpose()?;
        let mut attempt = Attempt {
            proof: self,
            tree: &tree,
            seat: seat.as_deref(),
            lease: lease.as_ref(),
            restarted: false,
        };
        let change = match attempt.run() {
            Ok(proof) => return Ok(proof),
            Err(Refusal::Check(error)) => return Err(error),
            Err(Refusal::Rollover(change)) => change,
        };
        eprintln!("{change}. Restarting verification with current packages.");
        attempt.restarted = true;
        match attempt.run() {
            Ok(proof) => Ok(proof),
            Err(Refusal::Check(error)) => Err(error),
            Err(Refusal::Rollover(change)) => {
                Err(format!("{change}. Guard refused after one restart."))
            }
        }
    }

    fn local(&self, tree: &str, seat: Option<&Path>) -> Result<Option<Descriptor>, String> {
        if self.latest || plumb::guard::staged(self.root, tree).is_err() {
            return Ok(None);
        }
        let index = crate::diagnostic::measure("source.prepare", || Index::new(self.root, tree))?;
        let prepared = self.prepare(&index, seat)?;
        Ok(self.reuse(&prepared, tree))
    }

    fn prepare(&self, index: &Index, seat: Option<&Path>) -> Result<Prepared, String> {
        let prepared = crate::diagnostic::measure("guard.prepare", || {
            Prepared::new(self.root, index, Selection::guard(self.latest), seat)
        })?;
        record(&prepared);
        Ok(prepared)
    }

    fn reuse(&self, prepared: &Prepared, tree: &str) -> Option<Descriptor> {
        prepared.reused(self.root, tree).inspect(|proof| {
            crate::diagnostic::event("guard.reused", || serde_json::json!({"proof":proof}));
        })
    }
}

struct Attempt<'a> {
    proof: &'a Proof<'a>,
    tree: &'a str,
    seat: Option<&'a Path>,
    lease: Option<&'a Lease>,
    restarted: bool,
}

impl Attempt<'_> {
    fn run(&self) -> Result<Descriptor, Refusal> {
        let root = self.proof.root;
        let tree = self.tree;
        let index = crate::diagnostic::measure("source.prepare", || Index::new(root, tree))?;
        let prepared = self.proof.prepare(&index, self.seat)?;
        if let Some(proof) = prepared.reused(root, tree) {
            if let Some(change) = changed(&index.root, prepared.channels.as_ref())? {
                return Err(Refusal::Rollover(change));
            }
            crate::diagnostic::event("guard.reused", || serde_json::json!({"proof":proof}));
            return Ok(proof);
        }
        let failure = self.verify(&index, &prepared)?;
        if let Some(change) = changed(&index.root, prepared.channels.as_ref())? {
            return Err(Refusal::Rollover(change));
        }
        if let Some(error) = failure {
            return Err(Refusal::Check(error));
        }
        tree::unchanged(&index)?;
        let actions = prepared
            .checks
            .into_iter()
            .map(|check| check.proof)
            .collect();
        let proof = Descriptor::new(root, tree.into(), actions)?;
        let proof = match prepared.resolution {
            Some(resolution) => proof.resolved(resolution)?,
            None => proof,
        };
        crate::diagnostic::event("guard.proof", || serde_json::json!({"proof":proof}));
        crate::execution::signal::check()?;
        plumb::guard::stage(root, &proof)?;
        Ok(proof)
    }

    fn verify(&self, index: &Index, prepared: &Prepared) -> Result<Option<String>, String> {
        let pending = prepared.checks.iter().filter(|check| !self.reused(check));
        let pending = pending.collect::<Vec<_>>();
        if pending.is_empty() {
            return Ok(None);
        }
        let session = self
            .lease
            .zip(prepared.context.as_ref())
            .map(|(lease, context)| {
                crate::command::cache::transport::Session::begin(
                    &index.root,
                    lease,
                    &prepared.material,
                    context,
                )
            })
            .transpose()?
            .flatten();
        if let Some(check) = pending.iter().find(|check| check.proof.name == "guard/web") {
            crate::diagnostic::measure("packages.fetch", || {
                plumb::packages::fetch(&index.root, &check.exclusions)
            })?;
            tree::unchanged(index)?;
        }
        let failure = self.execute(index, pending)?;
        if let Some(session) = session
            && let Err(error) = session.seal()
        {
            eprintln!("guard material: {error}");
        }
        Ok(failure)
    }

    fn reused(&self, check: &Check) -> bool {
        if !cache::contains(&check.proof) {
            return false;
        }
        if self.restarted {
            eprintln!("guard reused {}", check.proof.name);
        }
        crate::diagnostic::event(
            "action.reused",
            || serde_json::json!({"action":check.proof}),
        );
        true
    }

    fn execute(&self, index: &Index, pending: Vec<&Check>) -> Result<Option<String>, String> {
        for check in pending {
            let name = &check.proof.name;
            let mut span = crate::diagnostic::Span::new(super::suite::operation(name));
            if self.restarted {
                eprintln!("guard restarted {name}");
            } else {
                eprintln!("guard {name}");
            }
            if let Some(error) = self.commands(index, check)? {
                return Ok(Some(error));
            }
            crate::consumption::brand::verify(check.brand.as_ref(), &index.root)?;
            if let Some(protection) = &check.protection {
                protection.run(&index.root, check.execution.as_deref())?;
                tree::unchanged(index)?;
            }
            if let Some(lease) = self.lease {
                lease.settle();
            }
            crate::execution::signal::check()?;
            cache::record(&check.proof)?;
            span.complete(true);
        }
        Ok(None)
    }

    fn commands(&self, index: &Index, check: &Check) -> Result<Option<String>, String> {
        let name = &check.proof.name;
        let execution = tree::Execution {
            execution: check.execution.as_deref(),
            observation: crate::observation::bind("check", name),
        };
        for command in &check.commands {
            let result = tree::execute(&index.root, command, &execution);
            tree::unchanged(index)?;
            let Err(error) = result else { continue };
            if self.proof.latest && name == "guard/plumb" {
                return Ok(Some(error));
            }
            return Err(error);
        }
        Ok(None)
    }
}

fn record(prepared: &Prepared) {
    crate::diagnostic::event("guard.identity", || {
        serde_json::json!({
            "prepared_tree":prepared.resolution.as_ref().map(|resolution|&resolution.tree),
            "actions":prepared.checks.iter().map(|check|&check.proof).collect::<Vec<_>>(),
            "resolution":prepared.resolution,
            "image":prepared.context.as_ref().map(|context|&context.image)
        })
    });
}

fn changed(
    root: &std::path::Path,
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

pub(super) type Packages = (
    plumb::packages::Resolution,
    Option<plumb::packages::Observation>,
    Option<plumb::packages::Channels>,
);

pub(super) fn packages(index: &Index, selection: Selection) -> Result<Packages, String> {
    if !matches!(selection, Selection::Latest) {
        return Ok((
            plumb::packages::locked(&index.root, "local-locked")?,
            None,
            None,
        ));
    }
    let (locked, observed, channels) = crate::diagnostic::measure("packages.resolve", || {
        crate::command::packages::observed(&index.root, "ci-latest")
    })?;
    Ok((
        locked,
        Some(plumb::packages::Observation::new(observed)),
        Some(channels),
    ))
}
