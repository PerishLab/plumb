mod anchor;
mod catalog;
mod command;
mod consumption;
mod execution;
use execution::{cargo, observation};
mod judge;
mod shape;

use clap::{Parser, Subcommand};
use plumb::cli::Root;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "plumb",
    version = plumb::version!("PLUMB"),
    long_about = "Judge, prove, and deliver repositories.\n\nOptional Locus observation is disabled by default. Configure [locus] enabled = true in PLUMB_HOME/plumb.toml (default data home: .plumb under your home; LOCALAPPDATA/plumb on Windows). Set endpoint in that section to select a locus-api, otherwise LOCUS_API or http://127.0.0.1:43308 is used. Observation failures produce diagnostics and leave command results intact."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
pub(crate) enum Command {
    #[command(about = "Judge this repository against the skeleton and report every finding")]
    Doctor {
        #[command(flatten)]
        target: Root,
        #[arg(long)]
        json: bool,
    },
    #[command(
        about = "Project a clean topic branch onto its base and wait for its guard",
        long_about = plumb::seat::resource!("help/land.txt")
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
    #[command(
        about = "Prove the staged tree, exact CI HEAD, or one committed task boundary",
        long_about = plumb::seat::resource!("help/guard.txt")
    )]
    Guard {
        #[command(flatten)]
        target: Root,
        #[arg(long, requires = "head", requires = "write")]
        base: Option<String>,
        #[arg(long, requires = "base", requires = "write")]
        head: Option<String>,
        #[arg(long)]
        write: Vec<String>,
        #[arg(long, hide = true, conflicts_with_all = ["base", "head", "write"])]
        attach: Option<PathBuf>,
        #[arg(long, conflicts_with_all = ["base", "head", "write", "attach"])]
        refresh: bool,
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Resolve first-party manifests and locks to the latest stable",
        long_about = plumb::seat::resource!("help/lift.txt"))]
    Lift {
        #[command(flatten)]
        target: Root,
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Run one resumable Auto follow operation through the required Guard",
        long_about = plumb::seat::resource!("help/follow.txt"))]
    Follow {
        #[command(flatten)]
        target: Root,
        #[arg(
            long = "github-command",
            value_name = "GITHUB_COMMAND",
            default_value = "gh"
        )]
        github: PathBuf,
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
        deed: consumption::skill::Deed,
    },
    #[command(about = "Query the catalogued law by namespace, tag, standing, or owner")]
    Rule {
        #[command(subcommand)]
        deed: catalog::query::Deed,
    },
    #[command(about = "Read the release notes a version owes")]
    Changelog {
        #[command(flatten)]
        target: Root,
        #[arg(long)]
        version: Option<String>,
        #[arg(
            long,
            value_name = "DIR",
            help = "Prove a prepared release note against the diff budget of --version"
        )]
        prove: Option<PathBuf>,
    },
    #[command(about = "Project the repository side of the configuration Plumb carries")]
    Configuration {
        #[command(subcommand)]
        deed: consumption::configuration::Deed,
    },
    #[command(
        about = "Render the seats and file groups this repository declares",
        long_about = plumb::seat::resource!("help/layout.txt")
    )]
    Layout {
        #[command(flatten)]
        target: Root,
    },
    #[command(about = "Print the domain toolchain versions this Plumb carries")]
    Metadata {
        key: Option<String>,
        #[arg(long, conflicts_with = "key")]
        json: bool,
    },
    #[command(about = "Read what to do about a finding that names an entry")]
    Cookbook {
        entry: Option<String>,
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Record that a wayfinder was read against the authorities it points at")]
    Affirm {
        #[command(flatten)]
        target: Root,
        #[arg(long)]
        write: bool,
    },
    #[command(
        about = "Define and verify one immutable distribution marker",
        long_about = plumb::seat::resource!("help/release.txt")
    )]
    Release {
        #[command(subcommand)]
        deed: command::release::Deed,
    },
    #[command(
        about = "Dispatch every declared medium for one immutable release marker",
        long_about = plumb::seat::resource!("help/ship.txt")
    )]
    Ship {
        #[command(subcommand)]
        deed: command::ship::Deed,
    },
    #[command(
        about = "Move a release's changelog and skill onto Depot through wharf",
        long_about = plumb::seat::resource!("help/depot.txt")
    )]
    Depot {
        #[command(subcommand)]
        deed: command::depot::Deed,
    },
    #[command(about = "Inspect and reclaim idle Guard build caches",
        long_about = plumb::seat::resource!("help/cache.txt"))]
    Cache {
        #[command(subcommand)]
        deed: command::cache::Deed,
    },
}
fn main() {
    plumb::depot::carry(catalog::carried::FILES);
    if let Err(error) = plumb::identity!("PLUMB") {
        eprintln!("plumb: {error}");
        std::process::exit(1);
    }
    let command = match Cli::try_parse() {
        Ok(cli) => cli.command,
        Err(error) => {
            let code = error.exit_code();
            let _ = error.print();
            std::process::exit(code);
        }
    };
    let observation = observation::start(command.name());
    let prepared = plumb::identity::ready().and_then(|()| {
        consumption::prepare(&command).map_err(|error| {
            format!("cannot read installed rules: {error}\nrun plumb configuration install")
        })
    });
    let code = match prepared {
        Ok(()) => command::execute(command),
        Err(error) => {
            eprintln!("plumb: {error}");
            1
        }
    };
    let code = execution::signal::received().unwrap_or(code);
    if let Some(observation) = observation {
        observation.finish(code);
    }
    std::process::exit(code);
}

impl Command {
    fn name(&self) -> &'static str {
        match self {
            Self::Doctor { .. } => "doctor",
            Self::Land { .. } => "land",
            Self::Guard { .. } => "guard",
            Self::Lift { .. } => "lift",
            Self::Follow { .. } => "follow",
            Self::Radius { .. } => "radius",
            Self::Policy { .. } => "policy",
            Self::Skill { .. } => "skill",
            Self::Rule { .. } => "rule",
            Self::Changelog { .. } => "changelog",
            Self::Configuration { .. } => "configuration",
            Self::Layout { .. } => "layout",
            Self::Metadata { .. } => "metadata",
            Self::Cookbook { .. } => "cookbook",
            Self::Affirm { .. } => "affirm",
            Self::Release { .. } => "release",
            Self::Ship { .. } => "ship",
            Self::Depot { .. } => "depot",
            Self::Cache { .. } => "cache",
        }
    }
}
