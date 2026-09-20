use super::inventory::{digest, field};
use super::reuse::hash;
use super::reuse::{Keys, Record};
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
    #[arg(long, hide = true)]
    reuse: Option<String>,
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
    let authority = Authority::read()?;
    let source = if let Some(source) = input.reuse {
        if !source.starts_with("https://") {
            return Err("reused workload source must be an HTTPS URL".into());
        }
        source
    } else {
        if !input.workload.is_file() {
            return Err(format!(
                "workload {} is not a file",
                input.workload.display()
            ));
        }
        let object = format!("workloads/{}.tgz", digest(&input.workload)?);
        let source = authority.public(&object)?;
        authority.publish(&object, &input.workload, "application/gzip")?;
        source
    };
    let workload = Record::workload(input.action.clone(), &keys, source);
    authority.record(&workload)?;
    if let Some(publication) = &publication {
        authority.record(publication)?;
    }
    Ok(())
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

    fn record(&self, record: &Record) -> Result<(), String> {
        for (route, record) in record.routes() {
            let body = record.encode()?;
            match self.control.write(
                &route,
                &body,
                plumb::bucket::Policy {
                    media: "application/json",
                    cache: "public, max-age=31536000, immutable",
                },
                plumb::bucket::Condition::Absent,
            )? {
                plumb::bucket::Outcome::Held(()) => {}
                plumb::bucket::Outcome::Stale => self.held(&route, &record)?,
                plumb::bucket::Outcome::Missing => {
                    return Err(format!("workflow record {route} returned missing"));
                }
            }
        }
        Ok(())
    }

    fn held(&self, route: &str, record: &Record) -> Result<(), String> {
        match self.control.read(route)? {
            plumb::bucket::Outcome::Held(object) => {
                let mut held = Record::decode(&object.body)?;
                let mut wanted = record.clone();
                if route.starts_with("records/workload/") {
                    held.proof = None;
                    wanted.proof = None;
                    held.source = wanted.source.clone();
                }
                if held == wanted {
                    Ok(())
                } else {
                    Err(format!("workflow record {route} drifted"))
                }
            }
            _ => Err(format!("workflow record {route} vanished")),
        }
    }

    fn publish(&self, key: &str, path: &Path, content_type: &str) -> Result<(), String> {
        match self.control.head(key)? {
            plumb::bucket::Outcome::Held(()) => return Ok(()),
            plumb::bucket::Outcome::Missing => {}
            plumb::bucket::Outcome::Stale => {
                return Err("probing workflow workload returned stale".into());
            }
        }
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
