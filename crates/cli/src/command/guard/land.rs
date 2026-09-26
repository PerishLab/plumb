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
    pub plan: Option<PathBuf>,
    pub watch: bool,
    pub dry: bool,
    pub json: bool,
}

pub fn run(input: Input) -> i32 {
    if let Some(path) = &input.plan {
        return exact(&input, path);
    }
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
    match plumb::delivery::required(&input.root) {
        Ok(true) => {
            return rejected(
                &input.root,
                Refusal {
                    kind: "policy",
                    message: "plumb.toml requires a current delivery plan; pass --plan FILE"
                        .to_string(),
                },
                input.json,
            );
        }
        Ok(false) => {}
        Err(refusal) => return rejected(&input.root, refusal, input.json),
    }
    match plumb::land::run(request) {
        Ok(report) => landed(report, input.json),
        Err(refusal) => rejected(&input.root, refusal, input.json),
    }
}

fn exact(input: &Input, path: &std::path::Path) -> i32 {
    let plan = match plumb::delivery::read(path) {
        Ok(plan) => plan,
        Err(refusal) => return refused(&input.root, refusal, input.json),
    };
    match plumb::delivery::land(plumb::delivery::Landing {
        root: &input.root,
        plan: &plan,
        watch: input.watch,
    }) {
        Ok(report) => shown(report, input.json),
        Err(refusal) => refused(&input.root, refusal, input.json),
    }
}

fn shown(report: plumb::delivery::Report, json: bool) -> i32 {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("delivery land report should encode")
        );
        return 0;
    }
    println!("plumb land {}", report.root.display());
    println!();
    println!("  repository {}", report.repository);
    println!("  issue      #{}", report.issue.number);
    println!("  candidate  {}", report.candidate);
    println!("  pull       #{}", report.pull.number);
    println!("  node       {}", report.pull.node);
    println!("  url        {}", report.pull.url);
    println!();
    println!(
        "  {}",
        if report.merged {
            "exact planned candidate landed; Issue unchanged"
        } else {
            "exact planned pull is up; Issue unchanged"
        }
    );
    0
}

fn refused(root: &PathBuf, refusal: Refusal, json: bool) -> i32 {
    if json {
        let failed = Failed {
            schema: plumb::delivery::landing::SCHEMA,
            root,
            ok: false,
            refusal,
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&failed).expect("delivery land refusal should encode")
        );
    } else {
        eprintln!("plumb land: {refusal}");
    }
    1
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
