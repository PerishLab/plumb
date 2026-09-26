use clap::Subcommand;
use plumb::cli::Root;
use plumb::delivery::{Plan, Request};
use plumb::land::Refusal;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum Deed {
    #[command(about = "Validate one Issue-led repository delivery without changing forge state")]
    Delivery {
        #[command(flatten)]
        target: Root,
        #[arg(long, default_value = "main")]
        base: String,
        #[arg(long)]
        issue: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Serialize)]
struct Failed<'a> {
    schema: &'static str,
    root: &'a PathBuf,
    ok: bool,
    refusal: Refusal,
}

pub fn run(deed: Deed) -> i32 {
    match deed {
        Deed::Delivery {
            target,
            base,
            issue,
            json,
        } => {
            let root = PathBuf::from(target.root);
            match plumb::delivery::plan(Request {
                root: &root,
                base: &base,
                issue: &issue,
            }) {
                Ok(plan) => planned(plan, json),
                Err(refusal) => rejected(&root, refusal, json),
            }
        }
    }
}

fn planned(plan: Plan, json: bool) -> i32 {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&plan).expect("delivery plan should encode")
        );
        return 0;
    }
    println!("plumb plan delivery {}", plan.root.display());
    println!();
    println!("  repository  {}", plan.repository);
    println!("  issue       {}#{}", plan.repository, plan.issue.number);
    println!("  type        {}", plan.issue.kind);
    println!("  observed    {}", plan.observed);
    println!("  base        {}", plan.base);
    println!("  target      {}", plan.target);
    println!("  branch      {}", plan.branch);
    println!("  candidate   {}", plan.candidate);
    println!("  guard       {}", plan.guard.digest);
    println!();
    println!("  delivery declaration is current; no forge state changed");
    0
}

fn rejected(root: &PathBuf, refusal: Refusal, json: bool) -> i32 {
    if json {
        let failed = Failed {
            schema: plumb::delivery::SCHEMA,
            root,
            ok: false,
            refusal,
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&failed).expect("delivery refusal should encode")
        );
    } else {
        eprintln!("plumb plan delivery: {refusal}");
    }
    1
}
