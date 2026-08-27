use clap::Args;
use plumb::{
    cli::Root,
    forgejo::{Remote, git},
};
use std::path::PathBuf;

#[derive(Args)]
pub struct Release {
    #[command(flatten)]
    pub(super) target: Root,
    #[arg(
        long = "zone-id",
        help = "Exact Cloudflare zone ID that owns the release domain"
    )]
    zone: String,
    #[arg(long, help = "Mode-0600 seat for the writer's one-time secret")]
    escrow: Option<PathBuf>,
    #[arg(long, help = "Apply the ordered plan until every resource is ready")]
    pub(super) apply: bool,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Args)]
pub struct Workflow {
    #[command(flatten)]
    pub(super) target: Root,
    #[arg(long, help = "Public HTTPS hostname serving the shared inventory")]
    domain: String,
    #[arg(
        long = "zone-id",
        help = "Exact Cloudflare zone ID that owns the inventory domain"
    )]
    zone: String,
    #[arg(long, help = "Mode-0600 seat for the writer's one-time secret")]
    escrow: Option<PathBuf>,
    #[arg(long, help = "Apply the ordered plan until every resource is ready")]
    pub(super) apply: bool,
    #[arg(long)]
    pub(super) json: bool,
}

const RELEASE: [&str; 4] = [
    "RELEASE_PUBLISH_S3_ACCESS_KEY",
    "RELEASE_PUBLISH_S3_SECRET_KEY",
    "RELEASE_PUBLISH_S3_BUCKET",
    "RELEASE_PUBLISH_S3_ENDPOINT",
];

const WORKFLOW: [&str; 5] = [
    "WORKFLOW_INVENTORY_S3_ACCESS_KEY",
    "WORKFLOW_INVENTORY_S3_SECRET_KEY",
    "WORKFLOW_INVENTORY_S3_BUCKET",
    "WORKFLOW_INVENTORY_S3_ENDPOINT",
    "WORKFLOW_INVENTORY_URL",
];

#[derive(Clone)]
pub(super) enum Scope {
    Repository,
    Organization,
}

#[derive(Clone)]
pub(super) struct Model {
    pub profile: &'static str,
    pub product: String,
    pub bucket: String,
    pub domain: String,
    pub zone: String,
    pub escrow: PathBuf,
    pub remote: Remote,
    pub scope: Scope,
}

impl Model {
    pub fn read(input: Release) -> Result<Self, String> {
        let root = PathBuf::from(&input.target.root)
            .canonicalize()
            .map_err(|error| format!("cannot resolve {}: {error}", input.target.root))?;
        let spec = crate::shape::release::Spec::read(&root.join("plumb.toml"))?;
        let domain = spec
            .authority
            .strip_prefix("https://")
            .and_then(|held| held.split('/').next())
            .filter(|held| held.contains('.') && !held.contains(':'))
            .ok_or_else(|| "release authority must name one HTTPS hostname".to_string())?
            .to_string();
        if input.zone.trim().is_empty() {
            return Err("--zone-id cannot be empty".into());
        }
        let escrow = input
            .escrow
            .unwrap_or_else(|| root.join(".local/release-authority.env"));
        let escrow = if escrow.is_absolute() {
            escrow
        } else {
            root.join(escrow)
        };
        let remote = git::remote(&root, "")?;
        Ok(Self {
            profile: "release",
            bucket: format!("perish-{}-releases", spec.product),
            product: spec.product,
            domain,
            zone: input.zone,
            escrow,
            remote,
            scope: Scope::Repository,
        })
    }

    pub fn workflow(input: Workflow) -> Result<Self, String> {
        let root = PathBuf::from(&input.target.root)
            .canonicalize()
            .map_err(|error| format!("cannot resolve {}: {error}", input.target.root))?;
        let domain = hostname(&input.domain, "workflow inventory")?;
        if input.zone.trim().is_empty() {
            return Err("--zone-id cannot be empty".into());
        }
        let escrow = input
            .escrow
            .unwrap_or_else(|| root.join(".local/workflow-inventory-authority.env"));
        let escrow = if escrow.is_absolute() {
            escrow
        } else {
            root.join(escrow)
        };
        Ok(Self {
            profile: "workflow",
            bucket: "perish-workflow-inventory".into(),
            product: "inventory".into(),
            domain,
            zone: input.zone,
            escrow,
            remote: git::remote(&root, "")?,
            scope: Scope::Organization,
        })
    }

    pub fn writer(&self) -> String {
        format!("publish:{}", self.bucket)
    }

    pub fn temporary(&self) -> String {
        format!("tmp:{}", self.bucket)
    }

    pub fn secrets(&self) -> &'static [&'static str] {
        match self.scope {
            Scope::Repository => &RELEASE,
            Scope::Organization => &WORKFLOW,
        }
    }

    pub fn inventory(&self) -> String {
        format!("https://{}/inventory.json", self.domain)
    }

    pub fn organization(&self) -> Option<&str> {
        matches!(self.scope, Scope::Organization).then_some(self.remote.owner.as_str())
    }
}

fn hostname(value: &str, profile: &str) -> Result<String, String> {
    let held = value
        .strip_prefix("https://")
        .unwrap_or(value)
        .trim_end_matches('/');
    if held.contains('.') && !held.contains(['/', ':', '\r', '\n']) {
        Ok(held.to_string())
    } else {
        Err(format!("{profile} authority must name one HTTPS hostname"))
    }
}
