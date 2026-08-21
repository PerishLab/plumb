mod anchor;
mod command;
mod judge;
mod rules;
mod shape;
mod skill;

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
    #[command(about = "Judge this repository against the skeleton and report every finding")]
    Doctor {
        #[command(flatten)]
        target: Root,
        #[arg(long)]
        json: bool,
    },
    #[command(
        about = "Project a clean topic branch onto its base and wait for its guard",
        long_about = command::depot::carried("help/land.txt", plumb::seat::resource!("help/land.txt"))
    )]
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
    #[command(about = "Prove one committed delta stays inside the declared write paths")]
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
    #[command(about = "Report which repositories a candidate version would reach")]
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
    #[command(about = "Render the ectropy policy this repository's shape earns")]
    Policy {
        #[command(flatten)]
        target: Root,
        #[arg(long)]
        write: bool,
    },
    #[command(about = "Install, inspect, and upgrade the briefs a release carries")]
    Skill {
        #[command(subcommand)]
        deed: skill::Deed,
    },
    #[command(about = "Query the catalogued law by namespace, tag, standing, or owner")]
    Rule {
        #[command(subcommand)]
        deed: judge::catalog::query::Deed,
    },
    #[command(about = "Read the release notes a version owes")]
    Changelog {
        #[command(flatten)]
        target: Root,
        #[arg(long)]
        version: Option<String>,
    },
    #[command(about = "Render the workflows this repository declares")]
    Lane {
        #[command(flatten)]
        target: Root,
        #[arg(long)]
        write: bool,
    },
    #[command(
        about = "Render the seats and file groups this repository declares",
        long_about = command::depot::carried("help/layout.txt", plumb::seat::resource!("help/layout.txt"))
    )]
    Layout {
        #[command(flatten)]
        target: Root,
    },
    #[command(about = "Read what to do about a finding that names an entry")]
    Cookbook { entry: Option<String> },
    #[command(about = "Record that a wayfinder was read against the authorities it points at")]
    Affirm {
        #[command(flatten)]
        target: Root,
        #[arg(long)]
        write: bool,
    },
    #[command(about = "Publish, sync, and show the configuration Plumb carries")]
    Depot {
        #[command(subcommand)]
        deed: command::depot::Deed,
    },
    #[command(
        about = "Hold the truth cycle of a product release",
        long_about = command::depot::carried("help/release.txt", plumb::seat::resource!("help/release.txt"))
    )]
    Release {
        #[command(subcommand)]
        deed: command::release::Deed,
    },
    #[command(about = "Project one release onto one medium")]
    Ship {
        #[command(subcommand)]
        deed: command::ship::Deed,
    },

    #[command(
        about = "Destroy one declared delivery chain in a fixed order",
        long_about = command::depot::carried("help/retire.txt", plumb::seat::resource!("help/retire.txt"))
    )]
    Retire {
        #[command(flatten)]
        deed: command::retire::Deed,
    },
    #[command(about = "Ask and record what a rendered lane may skip")]
    Workflow {
        #[command(subcommand)]
        deed: command::workflow::Deed,
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
        } => command::land::run(command::land::Input {
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
            command::render::Seat::new(PathBuf::from(target.root)).policy(write)
        }
        Command::Skill { deed } => skill::run(deed),
        Command::Rule { deed } => judge::catalog::query::run(deed),
        Command::Changelog { target, version } => {
            command::render::Seat::new(PathBuf::from(target.root)).changelog(version)
        }
        Command::Lane { target, write } => {
            command::render::Seat::new(PathBuf::from(target.root)).lane(write)
        }
        Command::Layout { target } => {
            command::render::Seat::new(PathBuf::from(target.root)).layout()
        }
        Command::Cookbook { entry } => command::cookbook::run(entry),
        Command::Affirm { target, write } => {
            command::render::Seat::new(PathBuf::from(target.root)).affirm(write)
        }
        Command::Depot { deed } => command::depot::run(deed),
        Command::Release { deed } => command::release::run(deed),
        Command::Ship { deed } => command::ship::run(deed),
        Command::Retire { deed } => command::retire::run(deed),
        Command::Workflow { deed } => command::workflow::run(deed),
    }
}

fn main() {
    let command = match Cli::try_parse() {
        Ok(cli) => cli.command,
        Err(error) => {
            let code = error.exit_code();
            let run = command::audit::Run::start("parse");
            let _ = error.print();
            if let Some(run) = run {
                run.finish(code);
            }
            std::process::exit(code);
        }
    };
    let run = command::audit::Run::start(command.name());
    let code = execute(command);
    if let Some(run) = run {
        run.finish(code);
    }
    std::process::exit(code);
}
