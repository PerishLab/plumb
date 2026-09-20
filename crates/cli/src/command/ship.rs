use clap::Subcommand;

#[derive(Subcommand)]
pub enum Deed {
    #[command(about = "Dispatch every declared distribution medium for one release marker")]
    Dispatch {
        #[command(flatten)]
        options: super::operator::Dispatch,
    },
}

pub fn run(deed: Deed) -> i32 {
    let Deed::Dispatch { options } = deed;
    match super::operator::dispatch(options) {
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
