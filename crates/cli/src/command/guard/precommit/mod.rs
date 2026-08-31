mod action;
pub(crate) mod hook;
mod tree;

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
    pub json: bool,
}

pub fn run(input: Input) -> i32 {
    if let Some(message) = &input.attach {
        return plain(
            plumb::guard::attach(&input.root, message)
                .map(|proof| format!("attached guard proof {} for {}", proof.digest, proof.tree)),
        );
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

pub(crate) fn proof(root: &Path) -> Result<plumb::guard::Descriptor, String> {
    action::prove(root)
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
