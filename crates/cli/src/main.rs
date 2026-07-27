mod anchor;
mod dispatch;
mod judge;
mod pack;
mod rules;
mod shape;

use clap::{Parser, Subcommand};
use judge::{judge, show};
use plumb::cli::Root;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "plumb", version = plumb::version!("PLUMB"))]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Doctor {
        #[command(flatten)]
        target: Root,
    },
}

fn doctor(root: PathBuf) -> i32 {
    let held = shape::read(&root);
    println!("plumb doctor {}", root.display());
    println!();
    println!("  wrappers  {}", show(&held.wrappers));
    println!("  layout    {}", show(&held.dirs));
    println!("  lanes     {}", show(&held.lanes));
    println!("  publishes {}", show(&held.ships));
    println!(
        "  law       block={} path={} grants={}",
        held.block.unwrap_or(0),
        held.path.unwrap_or(0),
        show(&held.grants)
    );
    println!();
    let notes = judge(&held);
    if notes.is_empty() {
        println!("  true to the skeleton");
        return 0;
    }
    let mut wrong = 0;
    let mut blind = 0;
    for note in &notes {
        println!("  {}: {} [{}]", note.grade, note.line, note.dim);
        if note.grade == "out of true" {
            wrong += 1;
        }
        if note.grade == "blind" {
            blind += 1;
        }
    }
    println!();
    println!(
        "  {wrong} out of true, {} unknown to the skeleton, {blind} blind",
        notes.len() - wrong - blind
    );
    i32::from(wrong > 0)
}

fn main() {
    let code = match Cli::parse().command {
        Command::Doctor { target } => doctor(PathBuf::from(target.root)),
    };
    std::process::exit(code);
}
