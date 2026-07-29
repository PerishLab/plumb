mod anchor;
mod dispatch;
mod judge;
mod rules;
mod shape;
mod skill;
mod web;

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
    Policy {
        #[command(flatten)]
        target: Root,
        #[arg(long)]
        write: bool,
    },
    Skill {
        #[command(subcommand)]
        deed: skill::Deed,
    },
    Lock {
        #[command(flatten)]
        target: Root,
    },
    Changelog {
        #[command(flatten)]
        target: Root,
        #[arg(long)]
        version: Option<String>,
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
    if !held.sites.is_empty() {
        println!("  sites     {}", show(&held.sites));
    }
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

fn locks(root: PathBuf) -> i32 {
    let held = shape::read(&root);
    println!("plumb lock {}", root.display());
    println!();
    if held.locks.is_empty() {
        println!("  no lock is declared");
        return 0;
    }
    let seen = held.version.unwrap_or_default();
    for lock in &held.locks {
        match shape::seal(&root, lock) {
            Ok(hash) => println!(
                "  {} version = \"{seen}\"\n  {} hash = \"{hash}\"",
                lock.name, lock.name
            ),
            Err(why) => println!("  {} {why}", lock.name),
        }
    }
    println!();
    println!("  record these in plumb.toml only after reading what they cover");
    0
}

fn changelog(root: PathBuf, version: Option<String>) -> i32 {
    let held = version
        .or_else(|| shape::read(&root).version)
        .unwrap_or_default();
    println!("plumb changelog {}", root.display());
    println!();
    if held.is_empty() {
        println!("  no version to read: the repository declares none, so pass --version");
        return 1;
    }
    let seat = shape::changelog::seat(&root, &held);
    let found = shape::changelog::read(&root, &held);
    if found.is_empty() {
        println!(
            "  {} is documented in en and zh",
            shape::changelog::stamped(&held)
        );
        return 0;
    }
    println!("  {}", seat.display());
    for line in &found {
        println!("    {line}");
    }
    println!();
    println!("  a stable release is immutable; what it changed cannot be written afterwards");
    1
}

fn policy(root: PathBuf, write: bool) -> i32 {
    let path = root.join("ectropy.toml");
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => {
            eprintln!("plumb policy {}: {error}", root.display());
            return 1;
        }
    };
    let rendered = match shape::reconcile(&root, &text) {
        Ok(rendered) => rendered,
        Err(error) => {
            eprintln!("plumb policy {}: {error}", root.display());
            return 1;
        }
    };
    if !write {
        print!("{rendered}");
        return 0;
    }
    let draft = path.with_extension(format!("toml.tmp-{}", std::process::id()));
    if let Err(error) =
        std::fs::write(&draft, rendered).and_then(|()| std::fs::rename(&draft, &path))
    {
        let _ = std::fs::remove_file(&draft);
        eprintln!("plumb policy {}: {error}", root.display());
        return 1;
    }
    println!("plumb policy {}: wrote {}", root.display(), path.display());
    0
}

fn main() {
    let code = match Cli::parse().command {
        Command::Doctor { target } => doctor(PathBuf::from(target.root)),
        Command::Policy { target, write } => policy(PathBuf::from(target.root), write),
        Command::Skill { deed } => skill::run(deed),
        Command::Lock { target } => locks(PathBuf::from(target.root)),
        Command::Changelog { target, version } => changelog(PathBuf::from(target.root), version),
    };
    std::process::exit(code);
}
