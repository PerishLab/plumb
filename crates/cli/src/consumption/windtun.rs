use clap::Subcommand;
use plumb::cli::Root;
use std::path::Path;

#[derive(Subcommand)]
pub enum Deed {
    #[command(about = "Read and validate plumb.toml Windtun metadata without executing a suite")]
    Inspect {
        #[command(flatten)]
        target: Root,
        #[arg(long)]
        json: bool,
    },
}

pub fn run(deed: Deed) -> i32 {
    let Deed::Inspect { target, json } = deed;
    let result =
        plumb::windtun::inspect(Path::new(&target.root)).and_then(|held| render(&held, json));
    match result {
        Ok(text) => {
            println!("{text}");
            0
        }
        Err(error) => {
            eprintln!("plumb windtun: {error}");
            2
        }
    }
}

fn render(held: &plumb::windtun::Inspection, json: bool) -> Result<String, String> {
    if json {
        return serde_json::to_string(held).map_err(|error| error.to_string());
    }
    let state = if held.windtun.is_some() {
        "declared"
    } else {
        "undeclared"
    };
    let mut text = format!("windtun {state}");
    if let Some(manifest) = &held.manifest {
        text.push_str(&format!("\nmanifest {manifest}"));
    }
    Ok(text)
}
