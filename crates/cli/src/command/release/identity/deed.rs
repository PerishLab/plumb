use clap::Subcommand;

#[derive(Subcommand)]
pub enum Marker {
    #[command(about = "Resolve and report one immutable release marker as JSON")]
    Show {
        #[arg(long)]
        marker: String,
    },
    #[command(about = "Verify one immutable release marker")]
    Verify {
        #[arg(long)]
        marker: String,
    },
}

#[derive(Subcommand)]
pub enum Deed {
    #[command(about = "Adopt a tagged legacy release after external package readback")]
    Adopt {
        #[arg(default_value = ".")]
        root: String,
        #[arg(long)]
        version: String,
        #[arg(long = "dry-run")]
        dry: bool,
    },
    #[command(about = "Derive and prove this release plan, and report it as JSON")]
    Plan,
    #[command(about = "Report the media this product declares as JSON")]
    Surface,
    #[command(about = "Point the channel at the release this run published")]
    Activate,
    #[command(about = "Open or reuse the release line for a stable version and record its datum")]
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
    #[command(about = "Cherry-pick one commit onto a release line")]
    Pick {
        #[arg(long)]
        version: String,
        #[arg(long)]
        commit: String,
        #[arg(long = "dry-run")]
        dry: bool,
    },
    #[command(about = "Wall a release line and resolve the exact seal its promotion embeds")]
    Freeze {
        #[arg(long)]
        version: String,
        #[arg(long, default_value = "")]
        repo: String,
        #[arg(long = "dry-run")]
        dry: bool,
    },
    #[command(about = "Compile the capsule this release publishes")]
    Compile,
    #[command(about = "Wait for the canonical guard to prove this release commit")]
    Evidence,
    #[command(about = "Read a published release back and verify it against its seal")]
    Inspect,
    #[command(about = "Inspect or verify an immutable release marker")]
    Marker {
        #[command(subcommand)]
        deed: Marker,
    },
    #[command(about = "Merge a published stable line back so main holds its commit")]
    Rejoin {
        #[arg(long, default_value = "")]
        version: String,
        #[arg(long, default_value = "")]
        repo: String,
        #[arg(long = "dry-run")]
        dry: bool,
    },
    #[command(about = "Fetch the exact seal and digest a stable promotion embeds")]
    Promote,
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
        #[arg(long = "dry-run")]
        dry: bool,
    },
}
