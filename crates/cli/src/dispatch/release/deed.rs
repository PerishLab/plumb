use clap::Subcommand;

#[derive(Subcommand)]
pub enum Deed {
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
    #[command(about = "Print the canonical release authority this product declares")]
    Authority,
    #[command(about = "Derive the channel the release version names")]
    Channel,
    #[command(about = "Compile the capsule this release publishes")]
    Compile,
    #[command(about = "Wait for the canonical guard to prove this release commit")]
    Evidence,
    #[command(about = "Read a published release back and verify it against its seal")]
    Inspect,
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
    #[command(about = "Derive the release version the source ref names")]
    Reference,
    #[command(about = "Withdraw an unpublished release point")]
    Retract {
        #[arg(long)]
        version: String,
        #[arg(long = "dry-run")]
        dry: bool,
    },
    #[command(about = "Prove the source ref, commit, and channel agree before publication")]
    Source,
    #[command(about = "Stamp the release point at the head of its line")]
    Stamp {
        #[arg(long)]
        version: String,
        #[arg(long = "dry-run")]
        dry: bool,
    },
    #[command(about = "Report the media this product declares as JSON")]
    Surface,
}
