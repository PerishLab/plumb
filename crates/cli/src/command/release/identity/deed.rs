use clap::Subcommand;

#[derive(Subcommand)]
pub enum Deed {
    #[command(about = "Resolve and report one immutable release marker as JSON")]
    Show {
        #[arg(long)]
        marker: String,
        #[arg(long, help = "Verify the already-held Git snapshot without fetching")]
        held: bool,
    },
    #[command(about = "Verify one immutable release marker")]
    Verify {
        #[arg(long)]
        marker: String,
        #[arg(long, help = "Verify the already-held Git snapshot without fetching")]
        held: bool,
    },
    #[command(about = "Withdraw an unpublished release point")]
    Retract {
        #[arg(long)]
        version: String,
        #[arg(long = "dry-run")]
        dry: bool,
    },
    #[command(about = "Stamp the release point at the head of its line")]
    Stamp {
        #[arg(long)]
        version: String,
        #[arg(long, default_value = "github")]
        remote: String,
        #[arg(long = "dry-run")]
        dry: bool,
    },
    #[command(about = "Render the manager scripts one release version installs with")]
    Managers {
        #[arg(long)]
        version: String,
        #[arg(long)]
        out: std::path::PathBuf,
    },
}
