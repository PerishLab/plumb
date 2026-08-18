mod course;
mod datum;
mod line;
mod mark;
mod pick;
mod rejoined;
mod trigger;
mod value;

use clap::{Args, Subcommand};

#[derive(Args)]
#[group(skip)]
pub struct Dispatch {
    #[arg(long)]
    version: String,
    #[arg(long, default_value = "")]
    repo: String,
    #[arg(long)]
    watch: bool,
    #[arg(long = "dry-run")]
    dry: bool,
}

#[derive(Subcommand)]
pub enum Stable {
    Prepare {
        #[arg(long)]
        version: String,
        #[arg(long, default_value = "main")]
        from: String,
        #[arg(long, default_value = "")]
        repo: String,
        #[arg(long = "dry-run")]
        dry: bool,
    },
    Pick {
        #[arg(long)]
        version: String,
        #[arg(long)]
        commit: String,
        #[arg(long = "dry-run")]
        dry: bool,
    },
    Freeze {
        #[arg(long)]
        version: String,
        #[arg(long, default_value = "")]
        repo: String,
        #[arg(long = "dry-run")]
        dry: bool,
    },
    Rejoin {
        #[arg(long)]
        version: String,
        #[arg(long, default_value = "")]
        repo: String,
        #[arg(long = "dry-run")]
        dry: bool,
    },
    Retract {
        #[arg(long)]
        version: String,
        #[arg(long = "dry-run")]
        dry: bool,
    },
}

pub fn dispatch(options: Dispatch) -> Result<String, String> {
    trigger::run(options)
}

pub fn run(deed: Stable) -> i32 {
    match line::run(deed) {
        Ok(message) => {
            println!("{message}");
            0
        }
        Err(error) => {
            eprintln!("plumb stable: {error}");
            1
        }
    }
}
