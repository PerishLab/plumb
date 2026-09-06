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
    #[arg(
        long,
        help = "Recover a missing or inconsistent writer escrow before converging"
    )]
    recovery: bool,
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
        long,
        help = "Forgejo organization whose caller workflows consume the inventory"
    )]
    organization: Option<String>,
    #[arg(
        long,
        conflicts_with = "organization",
        help = "Bind inventory seats to this repository instead of an organization"
    )]
    repository: bool,
    #[arg(
        long = "zone-id",
        help = "Exact Cloudflare zone ID that owns the inventory domain"
    )]
    zone: String,
    #[arg(long, help = "Mode-0600 seat for the writer's one-time secret")]
    escrow: Option<PathBuf>,
    #[arg(
        long,
        help = "Recover a missing or inconsistent writer escrow before converging"
    )]
    recovery: bool,
    #[arg(long, help = "Apply the ordered plan until every resource is ready")]
    pub(super) apply: bool,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Args)]
pub struct Ship {
    #[command(flatten)]
    pub(super) target: Root,
    #[arg(
        long,
        help = "Mode-0600 seat for the controller writer's one-time secret"
    )]
    escrow: Option<PathBuf>,
    #[arg(
        long,
        help = "Recover a missing or inconsistent controller writer escrow before converging"
    )]
    recovery: bool,
    #[arg(long, help = "Apply the ordered plan until every resource is ready")]
    pub(super) apply: bool,
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Args)]
pub struct Depot {
    #[command(flatten)]
    target: Root,
    #[arg(
        long = "zone-id",
        help = "Exact Cloudflare zone ID that owns the depot domain"
    )]
    zone: String,
    #[arg(long, help = "Apply the ordered plan until every resource is ready")]
    pub(super) apply: bool,
    #[arg(long)]
    pub(super) json: bool,
}

pub(super) const RELEASE: [&str; 5] = [
    "RELEASE_PUBLISH_S3_ACCESS_KEY",
    "RELEASE_PUBLISH_S3_SECRET_KEY",
    "RELEASE_PUBLISH_S3_BUCKET",
    "RELEASE_PUBLISH_S3_ENDPOINT",
    "RELEASE_PUBLISH_FINGERPRINT",
];

pub(super) const WORKFLOW: [&str; 5] = [
    "WORKFLOW_INVENTORY_S3_ACCESS_KEY",
    "WORKFLOW_INVENTORY_S3_SECRET_KEY",
    "WORKFLOW_INVENTORY_S3_BUCKET",
    "WORKFLOW_INVENTORY_S3_ENDPOINT",
    "WORKFLOW_INVENTORY_URL",
];

pub(super) const SHIP: [&str; 4] = [
    "SHIP_PUBLISH_S3_ACCESS_KEY",
    "SHIP_PUBLISH_S3_SECRET_KEY",
    "SHIP_PUBLISH_S3_ENDPOINT",
    "SHIP_PUBLISH_FINGERPRINT",
];

#[derive(Clone)]
pub(super) enum Scope {
    Repository,
    Organization(String),
}

#[derive(Clone)]
pub(super) struct Model {
    pub profile: &'static str,
    pub product: String,
    pub bucket: String,
    pub buckets: Vec<String>,
    pub domain: String,
    pub zone: String,
    pub escrow: PathBuf,
    pub remote: Remote,
    pub scope: Scope,
    pub recovery: bool,
}

impl Model {
    pub fn read(input: Release) -> Result<Self, String> {
        let root = PathBuf::from(&input.target.root)
            .canonicalize()
            .map_err(|error| format!("cannot resolve {}: {error}", input.target.root))?;
        let spec = crate::shape::release::Spec::controller(&root)?;
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
            buckets: Vec::new(),
            product: spec.product,
            domain,
            zone: input.zone,
            escrow,
            remote,
            scope: Scope::Repository,
            recovery: input.recovery,
        })
    }

    pub fn workflow(input: Workflow) -> Result<Self, String> {
        let root = PathBuf::from(&input.target.root)
            .canonicalize()
            .map_err(|error| format!("cannot resolve {}: {error}", input.target.root))?;
        let domain = super::hostname(&input.domain, "workflow inventory")?;
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
        let remote = git::remote(&root, "")?;
        let scope = if input.repository {
            Scope::Repository
        } else {
            let organization = input.organization.unwrap_or_else(|| remote.owner.clone());
            let organization = organization.trim();
            if organization.is_empty() || organization.contains(['/', '\r', '\n']) {
                return Err("--organization must name one Forgejo organization".into());
            }
            Scope::Organization(organization.to_string())
        };
        Ok(Self {
            profile: "workflow",
            bucket: "perish-workflow-inventory".into(),
            buckets: Vec::new(),
            product: "inventory".into(),
            domain,
            zone: input.zone,
            escrow,
            remote,
            scope,
            recovery: input.recovery,
        })
    }

    pub fn ship(input: Ship) -> Result<Self, String> {
        let root = PathBuf::from(&input.target.root)
            .canonicalize()
            .map_err(|error| format!("cannot resolve {}: {error}", input.target.root))?;
        let escrow = input
            .escrow
            .unwrap_or_else(|| root.join(".local/ship-authority.env"));
        let escrow = if escrow.is_absolute() {
            escrow
        } else {
            root.join(escrow)
        };
        let catalog = std::fs::read_to_string(root.join("crates/cli/rules/products.toml"))
            .map_err(|error| format!("cannot read the ship product catalog: {error}"))?;
        let catalog: toml::Table = catalog
            .parse()
            .map_err(|error| format!("cannot parse the ship product catalog: {error}"))?;
        let buckets = catalog
            .get("product")
            .and_then(toml::Value::as_array)
            .ok_or_else(|| "ship product catalog names no products".to_string())?
            .iter()
            .map(|product| {
                product
                    .get("name")
                    .and_then(toml::Value::as_str)
                    .filter(|name| !name.is_empty())
                    .map(|name| format!("perish-{name}-releases"))
                    .ok_or_else(|| "ship product catalog carries an unnamed product".to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            profile: "ship",
            product: "release-buckets".into(),
            bucket: "perish-plumb-releases".into(),
            buckets,
            domain: String::new(),
            zone: String::new(),
            escrow,
            remote: git::remote(&root, "")?,
            scope: Scope::Repository,
            recovery: input.recovery,
        })
    }

    pub fn depot(input: Depot) -> Result<Self, String> {
        let root = PathBuf::from(&input.target.root)
            .canonicalize()
            .map_err(|error| format!("cannot resolve {}: {error}", input.target.root))?;
        let spec = crate::shape::release::Spec::controller(&root)?;
        let route = spec
            .route
            .ok_or_else(|| format!("product {} declares no depot route", spec.product))?;
        let domain = super::hostname(&route, "depot")?;
        if input.zone.trim().is_empty() {
            return Err("--zone-id cannot be empty".into());
        }
        Ok(Self {
            profile: "depot",
            product: spec.product,
            bucket: "perish-plumb-depot".into(),
            buckets: Vec::new(),
            domain,
            zone: input.zone,
            escrow: PathBuf::new(),
            remote: git::remote(&root, "")?,
            scope: Scope::Repository,
            recovery: false,
        })
    }
}
