mod inspect;
mod status;

use clap::{Args, Subcommand};
use plumb::cli::Root;
use plumb::lane::{Action, Address, Name};
use serde_json::Value;

#[derive(Subcommand)]
pub enum Deed {
    #[command(about = "Read one explicit local lane declaration and exact clean source")]
    Status(Status),
    #[command(about = "Read one anonymous immutable public record for an exact target")]
    Inspect(Inspection),
}

#[derive(Args)]
pub struct Selection {
    #[arg(long, help = "Exact platform app identity")]
    app: String,
    #[arg(long, help = "Independent flat lane name; no default or fallback")]
    lane: String,
}

impl Selection {
    fn address(&self, repository: String) -> Result<Address, String> {
        Address::new(repository, self.app.clone(), Name::read(&self.lane)?)
    }
}

#[derive(Args)]
pub struct Status {
    #[command(flatten)]
    target: Root,
    #[command(flatten)]
    selection: Selection,
    #[arg(long, value_name = "ACTION", value_parser = action, help = "Require a declared action without executing it")]
    require: Option<Action>,
    #[arg(long)]
    json: bool,
}

#[derive(Args)]
pub struct Inspection {
    #[command(flatten)]
    selection: Selection,
    #[arg(long, help = "Exact owner/repository coordinate to match")]
    repository: String,
    #[arg(long, help = "Prepared publication authority label")]
    authority: String,
    #[arg(long, help = "SHA256 of the canonical immutable public record")]
    digest: String,
    #[arg(long, help = "Explicit bare HTTPS origin for that authority")]
    origin: String,
    #[arg(
        long,
        requires = "revision",
        help = "Match one exact historical request, with --revision"
    )]
    request: Option<String>,
    #[arg(
        long,
        requires = "request",
        help = "Match the positive revision of --request"
    )]
    revision: Option<u64>,
    #[arg(long)]
    json: bool,
}

pub fn run(deed: Deed) -> i32 {
    let (json, result) = match deed {
        Deed::Status(options) => (options.json, status::read(options)),
        Deed::Inspect(options) => (options.json, inspect::read(options)),
    };
    match result {
        Ok(report) => {
            if json {
                println!("{report}");
            } else {
                render(&report);
            }
            0
        }
        Err(error) => {
            crate::observation::fault("lane.refused");
            eprintln!("plumb lane: {error}");
            1
        }
    }
}

fn action(value: &str) -> Result<Action, String> {
    serde_json::from_value(Value::String(value.into())).map_err(|_| {
        "expected inspect, build, publish, install, uninstall, deploy or dispose".into()
    })
}

fn render(report: &Value) {
    if report["schema"] == "plumb.lane.status/v1" {
        let declaration = &report["declaration"];
        println!("lane {}", declaration["target"]);
        println!("adaptor {}", declaration["adaptor"]);
        println!("capabilities {}", declaration["capabilities"]);
        println!("source {}", report["source"]);
        println!("current unknown: no live state was read");
    } else {
        println!("public evidence {}", report["reference"]);
        println!("historical record {}", report["record"]);
        println!(
            "This record does not establish current provider state or authorize an operation."
        );
    }
}
