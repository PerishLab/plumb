use clap::Subcommand;

#[derive(Subcommand)]
pub enum Deed {
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
        #[arg(long, default_value = "origin")]
        remote: String,
        #[arg(long = "dry-run")]
        dry: bool,
    },
    #[command(about = "Open the release line one version is stamped on")]
    Open {
        #[arg(long)]
        version: String,
        #[arg(long, default_value = "main")]
        from: String,
        #[arg(long, default_value = "origin")]
        remote: String,
        #[arg(long = "dry-run")]
        dry: bool,
    },
    #[command(about = "Close a release line once its stable marker is home")]
    Close {
        #[arg(long)]
        version: String,
        #[arg(long)]
        abandon: bool,
        #[arg(long, default_value = "origin")]
        remote: String,
        #[arg(long = "dry-run")]
        dry: bool,
    },
    #[command(about = "Report what the stable marker below a version still owes")]
    Owed {
        #[arg(long)]
        version: Option<String>,
        #[arg(long, default_value = "origin")]
        remote: String,
    },
    #[command(about = "Merge the standing stable marker home into main")]
    Rejoin {
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
