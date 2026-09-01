use super::inventory::{digest, field};
use super::reuse::hash;
use super::reuse::{Inventory, Keys, Record};
use clap::Args;
use std::{
    fs,
    path::{Path, PathBuf},
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
    control: plumb::bucket::Control,
    public: String,
}

impl Authority {
    fn read() -> Result<Self, String> {
        let held = plumb::rig::Rig::resolve(None)
            .map_err(|error| error.to_string())?
            .workflow
            .inventory;
        let access = field("PLUMB_WORKFLOW_INVENTORY_ACCESS", held.access)?;
        let secret = field("PLUMB_WORKFLOW_INVENTORY_SECRET", held.secret)?;
        let bucket = field("PLUMB_WORKFLOW_INVENTORY_BUCKET", held.bucket)?;
        let endpoint = field("PLUMB_WORKFLOW_INVENTORY_ENDPOINT", held.endpoint)?;
        let public = field("PLUMB_WORKFLOW_INVENTORY_URL", held.url)?;
        if !endpoint.starts_with("https://") && !endpoint.starts_with("http://127.0.0.1:") {
            return Err("PLUMB_WORKFLOW_INVENTORY_ENDPOINT must be HTTPS".into());
        }
        if !public.starts_with("https://") {
            return Err("PLUMB_WORKFLOW_INVENTORY_URL must be HTTPS".into());
        }
        Ok(Self {
            control: plumb::bucket::Control::new(access, secret, bucket, endpoint)?,
            public,
        })
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
        for attempt in 0..12 {
            let (mut inventory, etag) = self.inventory()?;
            inventory.record(workload.clone())?;
            if let Some(publication) = &publication {
                inventory.record(publication.clone())?;
            }
            let body = inventory.encode()?;
            let condition = etag.as_deref().map_or(
                plumb::bucket::Condition::Absent,
                plumb::bucket::Condition::Match,
            );
            match self.control.write(
                INVENTORY,
                &body,
                plumb::bucket::Policy {
                    media: "application/json",
                    cache: "no-store",
                },
                condition,
            )? {
                plumb::bucket::Outcome::Held(()) => return Ok(()),
                plumb::bucket::Outcome::Stale => {
                    let delay = 40 * (attempt + 1).min(10);
                    std::thread::sleep(std::time::Duration::from_millis(delay));
                    continue;
                }
                plumb::bucket::Outcome::Missing => {
                    return Err("publishing workflow inventory returned missing".into());
                }
            }
        }
        Err("workflow inventory remained busy after 12 attempts".into())
    }

    fn inventory(&self) -> Result<(Inventory, Option<String>), String> {
        match self.control.read(INVENTORY)? {
            plumb::bucket::Outcome::Missing => Ok((Inventory::empty(), None)),
            plumb::bucket::Outcome::Held(object) => {
                let inventory = Inventory::decode(&object.body)?;
                Ok((inventory, object.etag))
            }
            plumb::bucket::Outcome::Stale => {
                Err("reading workflow inventory returned stale".into())
            }
        }
    }

    fn publish(&self, key: &str, path: &Path, content_type: &str) -> Result<(), String> {
        let body = fs::read(path)
            .map_err(|error| format!("cannot read workload {}: {error}", path.display()))?;
        match self.control.write(
            key,
            &body,
            plumb::bucket::Policy {
                media: content_type,
                cache: "public, max-age=31536000, immutable",
            },
            plumb::bucket::Condition::Absent,
        )? {
            plumb::bucket::Outcome::Held(()) | plumb::bucket::Outcome::Stale => Ok(()),
            plumb::bucket::Outcome::Missing => {
                Err("publishing workflow workload returned missing".into())
            }
        }
    }
}
