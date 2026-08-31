use super::inventory::{absent, digest, failure, field, stale};
use super::reuse::hash;
use super::reuse::{Inventory, Keys, Record};
use clap::Args;
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const INVENTORY: &str = "inventory.json";

#[derive(Args)]
pub struct Input {
    #[arg(help = "Exact action name from the plan")]
    action: String,
    #[arg(
        long,
        help = "JSON workload, proof, and optional publication keys from the plan"
    )]
    keys: String,
    #[arg(long, help = "Reusable workload produced by the action")]
    workload: PathBuf,
    #[arg(
        long,
        help = "Verified public publication URL, when the action published"
    )]
    publication: Option<String>,
    #[arg(
        long,
        hide = true,
        help = "Marker-bound mutable projection carried by a publication"
    )]
    depot: Option<String>,
}

pub fn run(input: Input) -> i32 {
    match execute(input) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("plumb workflow record: {error}");
            1
        }
    }
}

fn execute(input: Input) -> Result<(), String> {
    if input.action.trim().is_empty() {
        return Err("action cannot be empty".into());
    }
    let keys: Keys = serde_json::from_str(&input.keys)
        .map_err(|error| format!("cannot parse --keys: {error}"))?;
    hash(&keys.workload)?;
    hash(&keys.proof)?;
    if let Some(publication) = &keys.publication {
        hash(publication)?;
    }
    let depot = input
        .depot
        .map(|depot| {
            let held: serde_json::Value = serde_json::from_str(&depot)
                .map_err(|error| format!("cannot parse --depot: {error}"))?;
            if held.is_object() {
                Ok(held)
            } else {
                Err("--depot must be one JSON object".to_string())
            }
        })
        .transpose()?;
    let publication = input
        .publication
        .map(|source| {
            if !source.starts_with("https://") {
                return Err("--publication must be an HTTPS URL".to_string());
            }
            Record::publication(input.action.clone(), &keys, source, depot)
                .ok_or_else(|| "--publication requires a publication key".to_string())
        })
        .transpose()?;
    if !input.workload.is_file() {
        return Err(format!(
            "workload {} is not a file",
            input.workload.display()
        ));
    }
    let authority = Authority::read()?;
    let object = format!("workloads/{}.tgz", digest(&input.workload)?);
    let source = authority.public(&object)?;
    authority.publish(&object, &input.workload, "application/gzip")?;
    let workload = Record::workload(input.action.clone(), &keys, source);
    authority.merge(workload, publication)
}

pub(in crate::command) struct Project<'a> {
    pub action: &'a str,
    pub keys: &'a str,
    pub workload: PathBuf,
    pub publication: Option<String>,
    pub depot: Option<serde_json::Value>,
}

pub(in crate::command) fn project(input: Project<'_>) -> Result<(), String> {
    execute(Input {
        action: input.action.to_string(),
        keys: input.keys.to_string(),
        workload: input.workload,
        publication: input.publication,
        depot: input.depot.map(|held| held.to_string()),
    })
}

struct Authority {
    access: String,
    secret: String,
    bucket: String,
    endpoint: String,
    public: String,
}

impl Authority {
    fn read() -> Result<Self, String> {
        let held = plumb::rig::Rig::resolve(None)
            .map_err(|error| error.to_string())?
            .workflow
            .inventory;
        let held = Self {
            access: field("PLUMB_WORKFLOW_INVENTORY_ACCESS", held.access)?,
            secret: field("PLUMB_WORKFLOW_INVENTORY_SECRET", held.secret)?,
            bucket: field("PLUMB_WORKFLOW_INVENTORY_BUCKET", held.bucket)?,
            endpoint: field("PLUMB_WORKFLOW_INVENTORY_ENDPOINT", held.endpoint)?,
            public: field("PLUMB_WORKFLOW_INVENTORY_URL", held.url)?,
        };
        if !held.endpoint.starts_with("https://") {
            return Err("PLUMB_WORKFLOW_INVENTORY_ENDPOINT must be HTTPS".into());
        }
        if !held.public.starts_with("https://") {
            return Err("PLUMB_WORKFLOW_INVENTORY_URL must be HTTPS".into());
        }
        Ok(held)
    }

    fn public(&self, key: &str) -> Result<String, String> {
        let suffix = format!("/{INVENTORY}");
        let base = self
            .public
            .strip_suffix(&suffix)
            .ok_or_else(|| format!("PLUMB_WORKFLOW_INVENTORY_URL must end in {suffix}"))?;
        Ok(format!("{base}/{key}"))
    }

    fn merge(&self, workload: Record, publication: Option<Record>) -> Result<(), String> {
        for _ in 0..8 {
            let (mut inventory, etag) = self.inventory()?;
            inventory.record(workload.clone())?;
            if let Some(publication) = &publication {
                inventory.record(publication.clone())?;
            }
            let seat = tempfile::NamedTempFile::new()
                .map_err(|error| format!("cannot stage workflow inventory: {error}"))?;
            fs::write(seat.path(), inventory.encode()?)
                .map_err(|error| format!("cannot stage workflow inventory: {error}"))?;
            let mut command = self.command();
            command
                .args(["put-object", "--bucket", &self.bucket, "--key", INVENTORY])
                .arg("--body")
                .arg(seat.path())
                .args([
                    "--content-type",
                    "application/json",
                    "--cache-control",
                    "no-store",
                ]);
            match etag {
                Some(etag) => {
                    command.args(["--if-match", &etag]);
                }
                None => {
                    command.args(["--if-none-match", "*"]);
                }
            }
            let output = command.arg("--no-cli-pager").output().map_err(|error| {
                format!("cannot run aws put-object for workflow inventory: {error}")
            })?;
            if output.status.success() {
                return Ok(());
            }
            if stale(&output) {
                continue;
            }
            return Err(failure("publish workflow inventory", &output));
        }
        Err("workflow inventory remained busy after 8 attempts".into())
    }

    fn inventory(&self) -> Result<(Inventory, Option<String>), String> {
        let seat = tempfile::tempdir()
            .map_err(|error| format!("cannot stage workflow inventory: {error}"))?;
        let path = seat.path().join(INVENTORY);
        let output = self
            .command()
            .args(["get-object", "--bucket", &self.bucket, "--key", INVENTORY])
            .arg(&path)
            .arg("--no-cli-pager")
            .output()
            .map_err(|error| {
                format!("cannot run aws get-object for workflow inventory: {error}")
            })?;
        if absent(&output) {
            return Ok((Inventory::empty(), None));
        }
        if !output.status.success() {
            return Err(failure("read workflow inventory", &output));
        }
        #[derive(Deserialize)]
        struct Get {
            #[serde(rename = "ETag")]
            etag: String,
        }
        let response: Get = serde_json::from_slice(&output.stdout)
            .map_err(|error| format!("cannot read workflow inventory ETag: {error}"))?;
        let inventory = Inventory::read(Some(&path))?;
        Ok((inventory, Some(response.etag)))
    }

    fn publish(&self, key: &str, path: &Path, content_type: &str) -> Result<(), String> {
        let output = self
            .command()
            .args(["put-object", "--bucket", &self.bucket, "--key", key])
            .arg("--body")
            .arg(path)
            .args([
                "--content-type",
                content_type,
                "--cache-control",
                "public, max-age=31536000, immutable",
                "--if-none-match",
                "*",
                "--no-cli-pager",
            ])
            .output()
            .map_err(|error| format!("cannot run aws put-object for workflow workload: {error}"))?;
        if output.status.success() || stale(&output) {
            Ok(())
        } else {
            Err(failure("publish workflow workload", &output))
        }
    }

    fn command(&self) -> Command {
        let mut command = Command::new("aws");
        command
            .env("AWS_ACCESS_KEY_ID", &self.access)
            .env("AWS_SECRET_ACCESS_KEY", &self.secret)
            .env("AWS_DEFAULT_REGION", "auto")
            .env("AWS_EC2_METADATA_DISABLED", "true")
            .arg("--endpoint-url")
            .arg(self.endpoint.trim_end_matches('/'))
            .arg("s3api");
        command
    }
}
