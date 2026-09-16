use super::store;
use crate::shape::depot as record;
use plumb::rig::Rig;
use std::path::Path;
use std::process::Output;

#[derive(clap::Args)]
pub struct Request {
    #[arg(default_value = ".")]
    root: String,
    #[arg(long)]
    marker: String,
    #[arg(long, required_unless_present = "promote", conflicts_with = "promote")]
    from: Option<String>,
    #[arg(
        long,
        help = "Upload and verify an immutable candidate without advancing latest",
        conflicts_with = "promote"
    )]
    stage: bool,
    #[arg(long, value_name = "GENERATION", requires = "expect")]
    promote: Option<String>,
    #[arg(long, value_name = "POINTER_SHA256_OR_ABSENT", requires = "promote")]
    expect: Option<String>,
    #[arg(
        long = "recovery-validator",
        help = "Use one exact local replacement for the selected released validator"
    )]
    recovery: Option<std::path::PathBuf>,
    #[arg(long = "dry-run")]
    dry: bool,
}

impl Request {
    pub fn run(&self) -> Result<String, String> {
        Tree(Path::new(&self.root)).publish(self)
    }
}

pub struct Tree<'a>(pub &'a Path);

impl Tree<'_> {
    pub fn publish(&self, request: &Request) -> Result<String, String> {
        let mut rig = Rig::resolve(None).map_err(|error| error.to_string())?;
        let marker = crate::command::release::ReleaseMarker::bound(self.0, &request.marker)?;
        let spec = marker.spec();
        let proof = marker.digest()?;
        if marker.commit != self.commit()? {
            return Err(format!(
                "release marker {} does not stand at HEAD",
                marker.marker
            ));
        }
        let depot = spec.derivative(plumb::depot::v3::Kind::Configuration)?;
        let product = crate::command::release::Product::new(spec);
        let release = product.depot();
        let binding = release.validator(&marker.version, true)?;
        let identity = plumb::depot::v3::Identity {
            product: marker.product.clone(),
            channel: marker.channel.clone(),
            version: marker.marker.clone(),
            marker: plumb::depot::v3::Marker {
                name: marker.marker.clone(),
                sha256: proof.clone(),
            },
            kind: plumb::depot::v3::Kind::Configuration,
        };
        let bundle = match &request.promote {
            Some(generation) => super::candidate::read(&depot.source, identity, generation)?,
            None => plumb::depot::v3::Bundle::read(
                Path::new(
                    request
                        .from
                        .as_deref()
                        .ok_or("configuration requires --from")?,
                ),
                identity,
            )?,
        };
        let plan = record::Batch::compatibility(
            &bundle,
            record::Draft {
                source: depot.source.clone(),
                release: binding.release.clone(),
                timestamp: super::super::clock::mark()?,
                commit: marker.commit.clone(),
            },
        )?;
        crate::command::release::validate_depot(
            spec,
            &binding,
            &plan,
            request.recovery.as_deref(),
        )?;
        let held = (|| {
            if request.dry {
                String::from_utf8(bundle.manifest.encode()?)
                    .map_err(|error| format!("depot manifest is not UTF-8: {error}"))
            } else {
                rig.depot.authority.load()?;
                let remote = store::Remote::new(&rig.depot.authority)?;
                if request.stage {
                    remote.stage(&bundle, &depot.source)
                } else if request.promote.is_some() {
                    remote.promote(
                        &bundle,
                        &depot.source,
                        (super::super::clock::ahead(0)?, request.expect.as_deref()),
                    )
                } else {
                    remote.publish(&bundle, &depot.source, super::super::clock::ahead(0)?)
                }
            }
        })();
        let after = crate::command::release::ReleaseMarker::bound(self.0, &marker.marker)?;
        if after.digest()? != proof {
            return Err(format!(
                "release marker {} drifted while depot was deriving configuration",
                marker.marker
            ));
        }
        held
    }

    pub fn commit(&self) -> Result<String, String> {
        let output = self.git(&["rev-parse", "HEAD"])?;
        if !output.status.success() {
            return Err("cannot read HEAD".to_string());
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn git(&self, args: &[&str]) -> Result<Output, String> {
        std::process::Command::new("git")
            .current_dir(self.0)
            .args(args)
            .output()
            .map_err(|error| format!("cannot run git: {error}"))
    }
}
