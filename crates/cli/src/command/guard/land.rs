use plumb::land::{Plan, Refusal, Report, Request};
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
    pub title: String,
    pub body: String,
    pub watch: bool,
    pub dry: bool,
    pub json: bool,
}

pub fn run(input: Input) -> i32 {
    let request = Request {
        root: &input.root,
        base: &input.base,
        title: &input.title,
        body: &input.body,
        watch: input.watch,
    };
    if input.dry {
        return match plumb::land::plan(request) {
            Ok(plan) => planned(plan, input.json),
            Err(refusal) => rejected(&input.root, refusal, input.json),
        };
    }
    match plumb::land::run(request) {
        Ok(report) => landed(report, input.json),
        Err(refusal) => rejected(&input.root, refusal, input.json),
    }
}

fn planned(plan: Plan, json: bool) -> i32 {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&plan).expect("land plan should encode")
        );
        return 0;
    }
    println!("plumb land {}", plan.root.display());
    println!();
    println!("  base       {}", plan.base);
    println!("  branch     {}", plan.branch);
    println!("  projection {}", plan.projection);
    println!();
    println!("  [dry-run] would run:");
    for step in &plan.steps {
        println!("    {step}");
    }
    0
}

fn landed(report: Report, json: bool) -> i32 {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("land report should encode")
        );
        return 0;
    }
    println!("plumb land {}", report.root.display());
    println!();
    println!("  base       {}", report.base);
    println!("  branch     {}", report.branch);
    println!("  projection {}", report.projection);
    println!("  source     {}", report.source);
    println!("  candidate  {}", report.candidate);
    println!("  pull       #{}", report.pull);
    if !report.url.is_empty() {
        println!("  url        {}", report.url);
    }
    match &report.synced {
        Some(seat) => println!("  synced     {}", seat.display()),
        None => println!("  synced     no separate {} seat", report.base),
    }
    println!();
    println!(
        "  {}",
        if report.merged {
            "landed; both branches retained"
        } else {
            "pull is up; guard not awaited"
        }
    );
    0
}

fn rejected(root: &PathBuf, refusal: Refusal, json: bool) -> i32 {
    if json {
        let failed = Failed {
            schema: plumb::land::SCHEMA,
            root,
            ok: false,
            refusal,
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&failed).expect("land refusal should encode")
        );
    } else {
        eprintln!("plumb land: {refusal}");
    }
    1
}
