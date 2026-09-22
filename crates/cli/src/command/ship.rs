use clap::Subcommand;

#[derive(Subcommand)]
pub enum Deed {
    #[command(about = "Dispatch every declared distribution medium for one release marker")]
    Dispatch {
        #[command(flatten)]
        options: super::operator::Dispatch,
    },
    #[command(
        about = "Report how far one release marker is distributed, from its distribution record"
    )]
    Status {
        #[arg(long)]
        marker: String,
    },
}

pub fn run(deed: Deed) -> i32 {
    let done = match deed {
        Deed::Dispatch { options } => super::operator::dispatch(options),
        Deed::Status { marker } => super::operator::status(&marker),
    };
    match done {
        Ok(message) => {
            println!("{message}");
            0
        }
        Err(error) => {
            eprintln!("plumb ship: {error}");
            1
        }
    }
}
