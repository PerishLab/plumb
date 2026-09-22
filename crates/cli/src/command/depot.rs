use clap::Subcommand;

#[derive(Subcommand)]
pub enum Deed {
    #[command(
        about = "Consign a release's changelog or skill to wharf's yard and lodge it on Depot"
    )]
    Consign {
        #[command(flatten)]
        options: super::operator::Consign,
    },
}

pub fn run(deed: Deed) -> i32 {
    let Deed::Consign { options } = deed;
    match super::operator::consign(options) {
        Ok(message) => {
            println!("{message}");
            0
        }
        Err(error) => {
            eprintln!("plumb depot: {error}");
            1
        }
    }
}
