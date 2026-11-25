mod artifact;
pub(super) mod context;
pub(super) mod inputs;
pub(super) mod provider;
pub(super) mod record;
mod request;
mod session;
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
pub(super) const FAMILY: &str = "guard-material-v1-";
pub(super) const MARKER: &str = "source.json";

#[derive(Serialize)]
struct World<'a> {
    schema: &'static str,
    repository: &'a str,
    home: &'a Path,
    image: Option<&'a str>,
    platform: &'a str,
}

#[derive(Serialize)]
struct Producer<'a> {
    plumb: &'a str,
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
    let world = World {
        schema: SCHEMA,
        repository: &repository,
        home: &home,
        image: context.as_ref().map(|context| context.image.as_str()),
        platform: &platform,
    };
    let producer = Producer {
        plumb: plumb::version!("PLUMB"),
    };
    crate::diagnostic::event("material.identity", || {
        serde_json::json!({
            "repository_key":plumb::depot::sha(repository.as_bytes()),"image":world.image,"platform":platform,
            "tree":tree,"producer":producer.plumb
        })
    });
    let world = digest(&world)?;
    let producer = digest(&producer)?;
    let family = format!("{FAMILY}{world}-");
    let restore = format!("{family}{producer}-");
    let slot = plumb::depot::sha(restore.as_bytes());
    let parcel = home
        .join("cache/transport")
        .join(&slot)
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
            family: &family,
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

fn digest<T: Serialize>(value: &T) -> Result<String, String> {
    let bytes = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    Ok(plumb::depot::sha(&bytes))
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

#[cfg(test)]
mod depot {
    use super::*;
    use serde::Deserialize;
    use std::process::Command;

    #[derive(Deserialize, Serialize)]
    struct Snapshot {
        depot: String,
        restore: String,
        key: String,
    }

    fn probe(home: &Path, root: &Path, output: &Path) -> Snapshot {
        let result = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "command::cache::transport::depot::invariant",
                "--nocapture",
            ])
            .env("PLUMB_HOME", home)
            .env("PLUMB_TRANSPORT_DEPOT_ROOT", root)
            .env("PLUMB_TRANSPORT_DEPOT_OUT", output)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        serde_json::from_slice(&std::fs::read(output).unwrap()).unwrap()
    }

    #[test]
    fn invariant() {
        if let (Some(root), Some(output)) = (
            std::env::var_os("PLUMB_TRANSPORT_DEPOT_ROOT"),
            std::env::var_os("PLUMB_TRANSPORT_DEPOT_OUT"),
        ) {
            plumb::depot::carry(crate::catalog::carried::FILES);
            let held = report(Path::new(&root), Action::Inspect).unwrap();
            let snapshot = Snapshot {
                depot: plumb::depot::rules().unwrap().mark().to_string(),
                restore: held.restore,
                key: held.key,
            };
            std::fs::write(output, serde_json::to_vec(&snapshot).unwrap()).unwrap();
            return;
        }
        let home = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        let git = |args: &[&str]| {
            let status = plumb::config::detached("git")
                .arg("-C")
                .arg(root.path())
                .args(args)
                .status()
                .unwrap();
            assert!(status.success());
        };
        git(&["init", "-q"]);
        git(&[
            "remote",
            "add",
            "origin",
            "https://github.com/Example/probe.git",
        ]);
        std::fs::create_dir(root.path().join("src")).unwrap();
        std::fs::write(
            root.path().join("Cargo.toml"),
            "[package]\nname='probe'\nversion='0.1.0'\nedition='2024'\n",
        )
        .unwrap();
        std::fs::write(
            root.path().join("src/lib.rs"),
            "pub fn value() -> u8 { 1 }\n",
        )
        .unwrap();
        let status = Command::new("cargo")
            .args(["generate-lockfile", "--offline"])
            .current_dir(root.path())
            .status()
            .unwrap();
        assert!(status.success());
        git(&["add", "."]);
        let first = probe(home.path(), root.path(), &home.path().join("first.json"));
        std::fs::create_dir_all(home.path().join("overlay")).unwrap();
        std::fs::write(home.path().join("overlay/probe.txt"), "overlay\n").unwrap();
        let next = probe(home.path(), root.path(), &home.path().join("next.json"));
        assert_ne!(first.depot, next.depot);
        assert_eq!(first.restore, next.restore);
        assert_eq!(first.key, next.key);
    }
}
