mod anchor;
mod dispatch;
mod judge;
mod rules;
mod shape;
mod skill;
mod web;

use clap::{Parser, Subcommand};
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
        #[arg(long)]
        json: bool,
    },
    Land {
        #[command(flatten)]
        target: Root,
        #[arg(long, default_value = "main")]
        base: String,
        #[arg(long, default_value = "")]
        title: String,
        #[arg(long, default_value = "")]
        body: String,
        #[arg(long = "no-watch", action = clap::ArgAction::SetFalse)]
        watch: bool,
        #[arg(long = "dry-run")]
        dry: bool,
        #[arg(long)]
        json: bool,
    },
    Precommit {
        #[command(flatten)]
        target: Root,
        #[arg(long)]
        base: String,
        #[arg(long)]
        head: String,
        #[arg(long, required = true)]
        write: Vec<String>,
        #[arg(long)]
        json: bool,
    },
    Radius {
        #[arg(long, required = true)]
        root: Vec<String>,
        #[arg(long)]
        product: String,
        #[arg(long)]
        candidate: String,
        #[arg(long)]
        json: bool,
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
    Rule {
        #[command(subcommand)]
        deed: judge::catalog::query::Deed,
    },
    Changelog {
        #[command(flatten)]
        target: Root,
        #[arg(long)]
        version: Option<String>,
    },
    Lane {
        #[command(flatten)]
        target: Root,
        #[arg(long)]
        write: bool,
    },
    Layout {
        #[command(flatten)]
        target: Root,
    },
    Cookbook {
        entry: Option<String>,
    },
    Affirm {
        #[command(flatten)]
        target: Root,
        #[arg(long)]
        write: bool,
    },
    Depot {
        #[command(subcommand)]
        deed: dispatch::depot::Deed,
    },
    Release {
        #[command(subcommand)]
        deed: dispatch::release::Deed,
    },
    Ship {
        #[command(subcommand)]
        deed: dispatch::ship::Deed,
    },

    Retire {
        #[command(flatten)]
        deed: dispatch::retire::Deed,
    },
    Workflow {
        #[command(subcommand)]
        deed: dispatch::workflow::Deed,
    },
}

impl Command {
    fn name(&self) -> &'static str {
        match self {
            Self::Doctor { .. } => "doctor",
            Self::Land { .. } => "land",
            Self::Precommit { .. } => "precommit",
            Self::Radius { .. } => "radius",
            Self::Policy { .. } => "policy",
            Self::Skill { .. } => "skill",
            Self::Rule { .. } => "rule",
            Self::Changelog { .. } => "changelog",
            Self::Lane { .. } => "lane",
            Self::Layout { .. } => "layout",
            Self::Cookbook { .. } => "cookbook",
            Self::Affirm { .. } => "affirm",
            Self::Depot { .. } => "depot",
            Self::Release { .. } => "release",
            Self::Ship { .. } => "ship",
            Self::Retire { .. } => "retire",
            Self::Workflow { .. } => "workflow",
        }
    }
}

fn execute(command: Command) -> i32 {
    match command {
        Command::Doctor { target, json } => judge::doctor::run(PathBuf::from(target.root), json),
        Command::Land {
            target,
            base,
            title,
            body,
            watch,
            dry,
            json,
        } => dispatch::land::run(dispatch::land::Input {
            root: PathBuf::from(target.root),
            base,
            title,
            body,
            watch,
            dry,
            json,
        }),
        Command::Precommit {
            target,
            base,
            head,
            write,
            json,
        } => judge::precommit::run(judge::precommit::Input {
            root: PathBuf::from(target.root),
            base,
            head,
            write,
            json,
        }),
        Command::Radius {
            root,
            product,
            candidate,
            json,
        } => judge::radius::run(judge::radius::Input {
            roots: root,
            product,
            candidate,
            json,
        }),
        Command::Policy { target, write } => {
            dispatch::command::Seat::new(PathBuf::from(target.root)).policy(write)
        }
        Command::Skill { deed } => skill::run(deed),
        Command::Rule { deed } => judge::catalog::query::run(deed),
        Command::Changelog { target, version } => {
            dispatch::command::Seat::new(PathBuf::from(target.root)).changelog(version)
        }
        Command::Lane { target, write } => {
            dispatch::command::Seat::new(PathBuf::from(target.root)).lane(write)
        }
        Command::Layout { target } => {
            dispatch::command::Seat::new(PathBuf::from(target.root)).layout()
        }
        Command::Cookbook { entry } => dispatch::cookbook::run(entry),
        Command::Affirm { target, write } => {
            dispatch::command::Seat::new(PathBuf::from(target.root)).affirm(write)
        }
        Command::Depot { deed } => dispatch::depot::run(deed),
        Command::Release { deed } => dispatch::release::run(deed),
        Command::Ship { deed } => dispatch::ship::run(deed),
        Command::Retire { deed } => dispatch::retire::run(deed),
        Command::Workflow { deed } => dispatch::workflow::run(deed),
    }
}

fn main() {
    let command = match Cli::try_parse() {
        Ok(cli) => cli.command,
        Err(error) => {
            let code = error.exit_code();
            let run = dispatch::audit::Run::start("parse");
            let _ = error.print();
            if let Some(run) = run {
                run.finish(code);
            }
            std::process::exit(code);
        }
    };
    let run = dispatch::audit::Run::start(command.name());
    let code = execute(command);
    if let Some(run) = run {
        run.finish(code);
    }
    std::process::exit(code);
}
