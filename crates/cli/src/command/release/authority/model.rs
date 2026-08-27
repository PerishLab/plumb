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

#[derive(Clone)]
pub(super) struct Model {
    pub product: String,
    pub bucket: String,
    pub domain: String,
    pub zone: String,
    pub escrow: PathBuf,
    pub remote: Remote,
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
            bucket: format!("perish-{}-releases", spec.product),
            product: spec.product,
            domain,
            zone: input.zone,
            escrow,
            remote,
        })
    }

    pub fn writer(&self) -> String {
        format!("publish:{}", self.bucket)
    }

    pub fn temporary(&self) -> String {
        format!("tmp:{}", self.bucket)
    }
}
