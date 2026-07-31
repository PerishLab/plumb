mod artifact;
mod cloud;
mod inspect;
mod model;
mod plan;
mod process;
mod reach;
mod settings;
mod ship;

use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum Deed {
    Plan {
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    Deploy {
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    Inspect {
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
}

pub fn run(deed: Deed) -> i32 {
    let result = match deed {
        Deed::Plan { root } => plan::run(&root),
        Deed::Deploy { root } => ship::run(&root),
        Deed::Inspect { root } => inspect::run(&root),
    };
    match result {
        Ok(message) => {
            println!("{message}");
            0
        }
        Err(error) => {
            eprintln!("plumb site: {error}");
            1
        }
    }
}
