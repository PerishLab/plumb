use plumb::boundary::{Refusal, Report, Request};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize)]
struct Failed<'a> {
    schema: &'static str,
    root: &'a PathBuf,
    ok: bool,
    refusal: Refusal,
}

pub struct Input {
    pub root: PathBuf,
    pub base: String,
    pub head: String,
    pub write: Vec<String>,
    pub json: bool,
}

pub fn run(input: Input) -> i32 {
    let request = Request {
        root: &input.root,
        base: &input.base,
        head: &input.head,
        write: &input.write,
    };
    match plumb::boundary::check(request) {
        Ok(report) => render(report, input.json),
        Err(refusal) => rejected(&input.root, refusal, input.json),
    }
}

fn render(report: Report, json: bool) -> i32 {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("precommit report should encode")
        );
    } else {
        println!("plumb precommit {}", report.root.display());
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
        eprintln!("plumb precommit {}: {refusal}", root.display());
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
