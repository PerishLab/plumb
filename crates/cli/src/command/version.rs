use clap::Subcommand;

#[derive(Subcommand)]
pub enum Deed {
    #[command(about = "Open or reuse the line for a stable version and project its identity")]
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
    #[command(about = "Cherry-pick one or more commits onto a version line atomically")]
    Pick {
        #[arg(long)]
        version: String,
        #[arg(long, required = true)]
        commit: Vec<String>,
        #[arg(long = "dry-run")]
        dry: bool,
    },
    #[command(about = "Freeze a version line after proving its exact promotion source")]
    Freeze {
        #[arg(long)]
        version: String,
        #[arg(long, default_value = "")]
        repo: String,
        #[arg(long = "dry-run")]
        dry: bool,
    },
    #[command(about = "Merge a shipped stable version back so main holds its commit")]
    Rejoin {
        #[arg(long)]
        version: String,
        #[arg(long, default_value = "")]
        repo: String,
        #[arg(long = "dry-run")]
        dry: bool,
    },
}

pub fn run(deed: Deed) -> i32 {
    match super::operator::line(deed) {
        Ok(message) => {
            println!("{message}");
            0
        }
        Err(error) => {
            eprintln!("plumb version: {error}");
            1
        }
    }
}
