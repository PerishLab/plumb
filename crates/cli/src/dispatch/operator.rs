mod line;
mod mark;
mod pick;
mod recovery;
mod trigger;
mod value;

use clap::{Args, Subcommand};

#[derive(Args)]
#[group(skip)]
pub struct Dispatch {
    #[arg(long)]
    version: String,
    #[arg(long, default_value = "beta")]
    promotion_channel: String,
    #[arg(long, default_value = "")]
    promotion_version: String,
    #[arg(long, default_value = "")]
    repo: String,
    #[arg(long)]
    watch: bool,
    #[arg(long = "dry-run")]
    dry: bool,
}

#[derive(Args)]
pub struct Arm {
    #[arg(long)]
    generator: String,
    #[arg(long)]
    release: String,
    #[arg(long)]
    actions: String,
    #[arg(long)]
    caller: String,
    #[arg(long = "dry-run")]
    dry: bool,
}

#[derive(Args)]
pub struct Promotion {
    #[arg(long)]
    caller: String,
    #[arg(long = "beta-sha256")]
    digest: String,
    #[arg(long = "dry-run")]
    dry: bool,
}

#[derive(Subcommand)]
pub enum Recovery {
    Arm(Arm),
    Beta {
        #[arg(long)]
        caller: String,
        #[arg(long = "dry-run")]
        dry: bool,
    },
    Stable(Promotion),
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

pub fn recovery(deed: Recovery, spec: &super::release::model::Spec) -> Result<String, String> {
    recovery::run(deed, spec)
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
