pub(super) mod context;
pub(super) mod inputs;
pub(super) mod provider;
pub(super) mod record;
mod request;
mod session;
pub(super) mod stamp;
pub(super) mod target;
#[cfg(test)]
mod tests;

pub(crate) use context::Context;
pub(crate) use session::Session;

use super::parcel::Receipt;
use super::transfer::{Action, Transfer};
use serde::Serialize;
use std::path::{Path, PathBuf};

const SCHEMA: &str = "plumb.guard-material/v1";
pub(super) const MARKER: &str = "source.json";

#[derive(Serialize)]
struct Identity<'a> {
    schema: &'static str,
    repository: &'a str,
    home: &'a Path,
    image: Option<&'a str>,
    platform: &'a str,
    plumb: &'a str,
    depot: &'a str,
}

#[derive(Serialize)]
struct Report {
    schema: &'static str,
    image: Option<String>,
    paths: Vec<PathBuf>,
    key: String,
    restore: String,
    artifact: String,
    populated: bool,
    receipt: Receipt,
}

#[derive(clap::Args)]
pub struct Input {
    #[arg(long, value_name = "NEW_FILE")]
    pub(crate) diagnostic: Option<PathBuf>,
    #[arg(default_value = ".")]
    root: PathBuf,
    #[arg(long)]
    json: bool,
    #[arg(long, conflicts_with = "save")]
    restore: bool,
    #[arg(long, requires = "restore", conflicts_with = "save")]
    fetch: bool,
    #[arg(long)]
    save: bool,
}

pub(super) fn run(input: Input) -> i32 {
    crate::diagnostic::event("material.mode", || {
        serde_json::json!({
            "restore":input.restore,"fetch":input.fetch,"save":input.save
        })
    });
    let action = if input.restore {
        Action::Restore { fetch: input.fetch }
    } else if input.save {
        Action::Save
    } else {
        Action::Inspect
    };
    match crate::diagnostic::measure("material.transport", || report(&input.root, action)) {
        Ok(report) if input.json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report).expect("transport report")
            );
            0
        }
        Ok(report) => {
            println!("{}", report.key);
            for path in report.paths {
                println!("{}", path.display());
            }
            0
        }
        Err(error) => {
            eprintln!("plumb cache transport: {error}");
            1
        }
    }
}

fn report(root: &Path, action: Action) -> Result<Report, String> {
    let tree = plumb::guard::tree(root)?;
    let home = super::home()?;
    let bucket = crate::command::guard::precommit::cargo::seat(root, &tree)?;
    let context = bucket
        .as_ref()
        .map(|_| Context::read(root))
        .transpose()?
        .flatten();
    let repository = crate::command::guard::precommit::cargo::identity(root)?;
    let platform = plumb::config::platform();
    let depot = plumb::depot::rules()?;
    let identity = Identity {
        schema: SCHEMA,
        repository: &repository,
        home: &home,
        image: context.as_ref().map(|context| context.image.as_str()),
        platform: &platform,
        plumb: plumb::version!("PLUMB"),
        depot: depot.mark(),
    };
    crate::diagnostic::event("material.identity", || {
        serde_json::json!({
            "repository_key":plumb::depot::sha(repository.as_bytes()),"image":identity.image,"platform":platform,
            "tree":tree,"producer":identity.plumb,"depot":identity.depot
        })
    });
    let bytes = serde_json::to_vec(&identity).map_err(|error| error.to_string())?;
    let digest = plumb::depot::sha(&bytes);
    let restore = format!("guard-material-v1-{digest}-");
    let parcel = home
        .join("cache/transport")
        .join(&digest)
        .join("material.tar.zst");
    let mut paths = Vec::new();
    let mut receipt = Receipt::default();
    if bucket.is_some() {
        paths.push(parcel.clone());
        receipt = Transfer {
            root,
            home: &home,
            context: context.as_ref(),
            parcel: &parcel,
            identity: &restore,
        }
        .run(action)?;
    }
    crate::diagnostic::event("material.receipt", || {
        serde_json::json!({
            "cargo":receipt.cargo,"compiler":receipt.compiler,
            "bytes":std::fs::metadata(&parcel).ok().map(|metadata|metadata.len()),
            "avoided_rebuild":null
        })
    });
    Ok(Report {
        schema: SCHEMA,
        image: context.map(|context| context.image),
        paths,
        key: format!("{restore}{tree}"),
        restore: restore.clone(),
        artifact: format!("{restore}{}", attempt()?),
        populated: receipt.any(),
        receipt,
    })
}

fn attempt() -> Result<u64, String> {
    match std::env::var("GITHUB_RUN_ATTEMPT") {
        Ok(value) => value
            .parse::<u64>()
            .ok()
            .filter(|value| *value != 0)
            .ok_or_else(|| "GITHUB_RUN_ATTEMPT must be positive".into()),
        Err(std::env::VarError::NotPresent) => Ok(1),
        Err(_) => Err("GITHUB_RUN_ATTEMPT is not text".into()),
    }
}
