use clap::Subcommand;
use plumb::cli::Root;
use std::path::Path;

#[derive(Subcommand)]
pub enum Deed {
    #[command(about = "Verify the latest declared qualification against current public inputs")]
    Check {
        #[arg(long)]
        repository: String,
        #[arg(long)]
        marker: String,
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Read and validate plumb.toml Windtun metadata without executing a suite")]
    Inspect {
        #[command(flatten)]
        target: Root,
        #[arg(long)]
        json: bool,
    },
}

pub fn run(deed: Deed) -> i32 {
    let (target, json) = match deed {
        Deed::Inspect { target, json } => (target, json),
        Deed::Check {
            repository,
            marker,
            json,
        } => return check(&repository, &marker, json),
    };
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

fn check(repository: &str, marker: &str, json: bool) -> i32 {
    use plumb::windtun::qualification::{Inspector, Remote, State};
    let report = Inspector::new(Remote::default()).inspect(repository, marker);
    let text = if json {
        serde_json::to_string(&report)
    } else {
        Ok(format!(
            "windtun {}: {}\nplatform {}\nqualification only; content association unassessed; promotion false",
            if report.qualified {
                "qualified"
            } else {
                "not qualified"
            },
            report
                .reason
                .as_deref()
                .unwrap_or("complete current declared credential"),
            report.platform
        ))
    };
    match text {
        Ok(text) => println!("{text}"),
        Err(error) => {
            eprintln!("plumb windtun: {error}");
            return 2;
        }
    }
    match report.state {
        State::Qualified => 0,
        State::Refused => 2,
        State::Unknown => 3,
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
