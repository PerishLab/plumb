use clap::Subcommand;

#[derive(Subcommand)]
pub enum Deed {
    #[command(about = "Read current protected release semantics requirements", long_about = plumb::seat::resource!("help/release/policy.txt"))]
    Policy,
    #[command(about = "Read an independently authenticated semantic review", long_about = plumb::seat::resource!("help/release/review.txt"))]
    Review {
        #[arg(long)]
        product: String,
        #[arg(long)]
        reference: String,
        #[arg(long, value_enum)]
        purpose: Purpose,
    },
    #[command(about = "Inspect release data or authenticate exact semantic records", long_about = plumb::seat::resource!("help/release/inspect.txt"))]
    Inspect {
        #[arg(long)]
        input: std::path::PathBuf,
        #[arg(long, value_enum, default_value = "data")]
        purpose: Inspection,
        #[arg(long, required_if_eq_any([("purpose", "materials"), ("purpose", "issuer"), ("purpose", "baseline")]))]
        product: Option<String>,
    },
    #[command(about = "Collect exact public source and formal Scene review inputs", long_about = plumb::seat::resource!("help/release/collect.txt"))]
    Collect {
        #[arg(long)]
        repository: String,
        #[arg(long)]
        commit: String,
    },
    #[command(about = "Assess supplied release boundary observations without publishing", long_about = plumb::seat::resource!("help/release/assess.txt"))]
    Assess {
        #[arg(long)]
        input: std::path::PathBuf,
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
        #[arg(long, default_value = "origin")]
        remote: String,
        #[arg(long = "dry-run")]
        dry: bool,
    },
    #[command(about = "Open the release line one version is stamped on")]
    Open {
        #[arg(long)]
        version: String,
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
    #[command(about = "Report the Guard authority this binary's proofs carry")]
    Authority {
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Report whether main holds the standing stable marker's changes")]
    Rejoin,
}

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum Purpose {
    Boundary,
    Adoption,
}

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum Inspection {
    Data,
    Materials,
    Issuer,
    Baseline,
}
