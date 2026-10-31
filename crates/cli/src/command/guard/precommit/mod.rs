mod action;
pub(crate) mod branch;
mod cache;
pub(crate) mod cargo;
mod environment;
pub(crate) mod hook;
pub(crate) mod tree;
mod web;
mod world;

use plumb::boundary::{Refusal, Report, Request};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
struct Failed<'a> {
    schema: &'static str,
    root: &'a PathBuf,
    ok: bool,
    refusal: Refusal,
}

pub struct Input {
    pub root: PathBuf,
    pub base: Option<String>,
    pub head: Option<String>,
    pub write: Vec<String>,
    pub attach: Option<PathBuf>,
    pub refresh: bool,
    pub json: bool,
}

#[locus::trace(with = crate::observation::view())]
pub fn run(input: Input) -> i32 {
    let runtime = match super::runtime::Selection::read() {
        Ok(runtime) => runtime,
        Err(error) => return super::runtime::refused(&input.root, &error, input.json),
    };
    if let Some(runtime) = runtime {
        if input.base.is_some() || input.head.is_some() || !input.write.is_empty() {
            return super::runtime::refused(
                &input.root,
                "PLUMB_GUARD_STRENGTH and PLUMB_GUARD_BOUNDARY disagree with the explicit --base, --head, or --write boundary",
                input.json,
            );
        }
        if input.attach.is_some() || input.refresh {
            return super::runtime::refused(
                &input.root,
                "runtime Guard selectors cannot be combined with --attach or --refresh",
                input.json,
            );
        }
        return runtime.run(&input.root, input.json);
    }
    if let Some(message) = &input.attach {
        return plain(
            action::prove(&input.root)
                .and_then(|_| plumb::guard::attach(&input.root, message))
                .map(|proof| format!("attached guard proof {} for {}", proof.digest, proof.tree)),
        );
    }
    if input.refresh {
        return branch::refresh(&input.root, input.json);
    }
    match (&input.base, &input.head) {
        (Some(base), Some(head)) => boundary(&input, base, head),
        (None, None) if input.write.is_empty() => Staged(&input.root).run(input.json),
        _ => {
            eprintln!("plumb guard: --base, --head, and --write form one task boundary");
            1
        }
    }
}

pub(super) fn prove(root: &Path) -> Result<plumb::guard::Descriptor, String> {
    action::resolved(root)
}

#[derive(Clone, Copy)]
enum Selection {
    Local,
    Latest,
    Compiler,
}

impl Selection {
    fn guard(latest: bool) -> Self {
        if latest { Self::Latest } else { Self::Local }
    }

    fn checks(
        self,
        tree: &super::workflow::tree::Tree,
        product: &str,
    ) -> Result<Vec<action::Preparation>, String> {
        let mut checks = action::Catalog { tree, product }.checks()?;
        if matches!(self, Self::Compiler) {
            checks.retain(|check| check.name == "guard/rust");
        }
        Ok(checks)
    }
}

pub(crate) struct Compiler<'a> {
    pub root: &'a Path,
    pub all: bool,
}

impl Compiler<'_> {
    pub(crate) fn resolve(&self) -> Result<crate::execution::lock::Report, String> {
        let root = self.root;
        crate::catalog::prepare()?;
        crate::catalog::set::prepare()?;
        let source = plumb::guard::tree(root)?;
        let lease = cargo::Lease::new(root)?;
        let index = tree::Index::new(root, &source)?;
        let prepared = world::Prepared::new(root, &index, Selection::Compiler, Some(&lease.root))?;
        let check = prepared
            .checks
            .iter()
            .find(|check| check.proof.name == "guard/rust")
            .ok_or("lock resolve requires the standard guard/rust action")?;
        let execution = check
            .execution
            .as_ref()
            .ok_or("lock resolve requires managed Cargo execution")?;
        let scope = crate::execution::lock::Scope {
            all: self.all,
            source,
            tree: tree::git(
                &index.root,
                &["rev-parse", "HEAD^{tree}"],
                "read compiled lock source",
            )?,
            resolution: prepared.resolution,
        };
        let report = crate::execution::lock::resolve(&index.root, execution, scope)?;
        tree::unchanged(&index)?;
        lease.settle();
        Ok(report)
    }
}

pub(crate) fn hooks(root: &Path) -> Vec<hook::Finding> {
    hook::audit(root)
}

pub(crate) fn project(root: &Path) -> Result<Option<String>, String> {
    hook::project(root)
}

fn boundary(input: &Input, base: &str, head: &str) -> i32 {
    let request = Request {
        root: &input.root,
        base,
        head,
        write: &input.write,
    };
    match plumb::boundary::check(request) {
        Ok(report) => render(report, input.json),
        Err(refusal) => rejected(&input.root, refusal, input.json),
    }
}

struct Staged<'a>(&'a Path);

impl Staged<'_> {
    fn run(&self, json: bool) -> i32 {
        if let Err(finding) = branch::inspect(self.0) {
            finding.render(json);
            return 1;
        }
        match action::prove(self.0) {
            Ok(proof) => {
                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&proof).expect("guard proof should encode")
                    );
                } else {
                    println!("plumb guard {}", self.0.display());
                    println!();
                    println!("  tree    {}", proof.tree);
                    println!("  proof   {}", proof.digest);
                    println!(
                        "  actions {}",
                        proof
                            .actions
                            .iter()
                            .map(|action| action.name.as_str())
                            .collect::<Vec<_>>()
                            .join(" ")
                    );
                    println!();
                    println!("  staged tree proved; commit-msg will carry the proof");
                }
                0
            }
            Err(error) => {
                eprintln!("plumb guard: {error}");
                1
            }
        }
    }
}

fn plain(result: Result<String, String>) -> i32 {
    match result {
        Ok(message) => {
            println!("{message}");
            0
        }
        Err(error) => {
            eprintln!("plumb guard: {error}");
            1
        }
    }
}

fn render(report: Report, json: bool) -> i32 {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("precommit report should encode")
        );
    } else {
        println!("plumb guard {}", report.root.display());
        println!();
        println!("  base    {}", report.base);
        println!("  head    {}", report.head);
        println!("  write   {}", report.write.join(" "));
        println!("  changed {}", list(&report.changed));
        println!("  outside {}", list(&report.outside));
        println!();
        println!(
            "  {}",
            if report.ok {
                "within the boundary"
            } else {
                "outside the boundary"
            }
        );
    }
    i32::from(!report.ok)
}

fn rejected(root: &PathBuf, refusal: Refusal, json: bool) -> i32 {
    crate::observation::fault(refusal.kind);
    if json {
        let report = Failed {
            schema: plumb::boundary::SCHEMA,
            root,
            ok: false,
            refusal,
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("precommit refusal should encode")
        );
    } else {
        eprintln!("plumb guard {}: {refusal}", root.display());
    }
    1
}

fn list(paths: &[String]) -> String {
    if paths.is_empty() {
        "-".to_owned()
    } else {
        paths.join(" ")
    }
}
