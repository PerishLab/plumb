use clap::Subcommand;
use std::path::Path;

#[derive(Subcommand)]
pub enum Deed {
    #[command(about = "Project the Git guard hooks this Plumb carries into a repository")]
    Install {
        #[arg(default_value = ".")]
        root: String,
    },
}

pub fn run(deed: Deed) -> i32 {
    let held = match deed {
        Deed::Install { root } => install(Path::new(&root)),
    };
    match held {
        Ok(message) => {
            println!("{message}");
            0
        }
        Err(error) => {
            eprintln!("plumb configuration: {error}");
            1
        }
    }
}

fn install(root: &Path) -> Result<String, String> {
    if let Some(raw) = super::pin()? {
        let source = super::origin()?;
        let source = source.as_deref().unwrap_or(super::DEPOT);
        let pinned = plumb::depot::v3::Pinned::parse(&raw, source)?;
        let home = super::home()?;
        pinned.cached(&home, source)?;
    }
    Ok(
        crate::command::precommit::project(root)?.unwrap_or_else(|| {
            format!(
                "{} carries no plumb.toml; no hooks projected",
                root.display()
            )
        }),
    )
}
