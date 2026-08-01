mod line;
mod pick;
mod trigger;
mod value;

use clap::{Args, Subcommand};

#[derive(Args)]
#[group(skip)]
pub struct Dispatch {
    #[arg(long)]
    channel: String,
    #[arg(long)]
    version: String,
    #[arg(long, default_value = "beta")]
    promotion_channel: String,
    #[arg(long, default_value = "")]
    promotion_version: String,
    #[arg(long, default_value = "")]
    repo: String,
    #[arg(long, default_value = "")]
    r#ref: String,
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
    Packport {
        #[arg(long)]
        version: String,
        #[arg(long, default_value = "")]
        repo: String,
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
