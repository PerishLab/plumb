use super::super::truth::{record, verify};
use super::binding::{self, Identity};
use super::promotion;
use super::seat::command;
use crate::command::release::{Deed, channel};
use crate::shape::release::Spec;
use plumb::forgejo::git;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::command) struct Descriptor {
    schema: &'static str,
    pub(in crate::command) product: String,
    pub(in crate::command) repository: String,
    pub(in crate::command) authority: String,
    pub(in crate::command) marker: String,
    pub(in crate::command) version: String,
    pub(in crate::command) channel: String,
    pub(in crate::command) commit: String,
    pub(in crate::command) tree: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in crate::command) configuration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in crate::command) profile: Option<String>,
    #[serde(skip)]
    spec: Box<Spec>,
    state: &'static str,
    datum: binding::Datum,
    #[serde(skip_serializing_if = "Option::is_none")]
    promotion: Option<Exact>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Exact {
    marker: String,
    channel: String,
    seal: Seal,
}

#[derive(Serialize)]
struct Seal {
    url: String,
    sha256: String,
}

pub(super) struct Seat {
    pub(super) root: PathBuf,
    pub(super) repository: String,
}

pub fn run(deed: Deed) -> Result<String, String> {
    let (name, show, held) = match deed {
        Deed::Show { marker, held } => (marker, true, held),
        Deed::Verify { marker, held } => (marker, false, held),
        Deed::Stamp { .. } => {
            return Err("a marker mutation reached marker inspection".into());
        }
    };
    let marker = resolve(&name, !held)?;
    if show {
        let text = serde_json::to_string_pretty(&marker).map_err(|error| error.to_string())?;
        Ok(format!("{text}\n"))
    } else {
        Ok(format!(
            "verified release marker {} at {} ({})",
            marker.marker,
            marker.commit,
            marker.digest()?
        ))
    }
}

pub(in crate::command) fn resolve(raw: &str, refresh: bool) -> Result<Descriptor, String> {
    Seat::open()?.resolve(raw, refresh, None)
}

pub(in crate::command) fn bound(root: &Path, raw: &str) -> Result<Descriptor, String> {
    Seat::new(root.to_path_buf())?.resolve(raw, true, None)
}

impl Descriptor {
    pub(in crate::command) fn recorded(root: &Path, raw: &str) -> Result<Option<Self>, String> {
        let seat = Seat::new(root.to_path_buf())?;
        let reference = format!("refs/tags/{raw}");
        let message = seat.read(["for-each-ref", "--format=%(contents)", &reference])?;
        if !matches!(
            binding::protocol(message.trim())?,
            Some("plumb.release-marker/v3" | "plumb.release-marker/v4")
        ) {
            return Ok(None);
        }
        seat.resolve(raw, false, Some(&reference)).map(Some)
    }

    pub(in crate::command) fn independent(&self) -> bool {
        matches!(
            self.schema,
            "plumb.release-marker/v3" | "plumb.release-marker/v4"
        )
    }

    pub(in crate::command) fn base(&self) -> &str {
        self.marker.split('-').next().unwrap_or(&self.marker)
    }

    pub(in crate::command) fn digest(&self) -> Result<String, String> {
        serde_json::to_vec(self)
            .map(|bytes| record::sha(&bytes))
            .map_err(|error| error.to_string())
    }

    pub(in crate::command) fn spec(&self) -> &Spec {
        &self.spec
    }
}

impl Seat {
    fn open() -> Result<Self, String> {
        let root = git::root()?;
        Self::new(root)
    }

    fn new(root: PathBuf) -> Result<Self, String> {
        let remote = git::remote(&root, "")?;
        Ok(Self {
            root,
            repository: format!("{}/{}", remote.owner, remote.repo),
        })
    }

    fn resolve(
        &self,
        raw: &str,
        refresh: bool,
        standing: Option<&str>,
    ) -> Result<Descriptor, String> {
        let marker = named(raw);
        let channel = channel::channel(&marker)
            .map_err(|error| format!("invalid release marker {marker}: {error}"))?;
        refresh.then(|| self.fetch()).transpose()?;
        let identity = self.identity(&marker)?;
        let commit = self.read(["rev-parse", &format!("{marker}^{{commit}}")])?;
        let tree = self.read(["rev-parse", &format!("{marker}^{{tree}}")])?;
        match plumb::guard::commit(&self.root, &commit) {
            Ok(proof) if proof.tree != tree => {
                return Err(format!(
                    "release marker {marker} tree {tree} disagrees with guard proof {}",
                    proof.tree
                ));
            }
            Ok(_) => {}
            Err(error)
                if matches!(
                    identity.schema,
                    "plumb.release-marker/v3" | "plumb.release-marker/v4"
                ) || guarded(&identity.product, &marker) =>
            {
                return Err(format!(
                    "release marker {marker} has no valid guard proof: {error}"
                ));
            }
            Err(_) => {}
        }
        let version = marker.split('-').next().unwrap_or(&marker).to_string();
        let line = super::standing::line(&version, refresh);
        self.stood(&marker, standing.unwrap_or(&line), &commit)?;
        let datum = binding::datum(&self.root, &version, &commit, identity.datum.as_deref())?;
        let promotion = if channel == "stable"
            && !matches!(
                identity.schema,
                "plumb.release-marker/v3" | "plumb.release-marker/v4"
            ) {
            Some(self.promotion(&identity.product, &identity.authority, &version, &commit)?)
        } else {
            None
        };
        Ok(Descriptor {
            schema: identity.schema,
            product: identity.product,
            repository: self.repository.clone(),
            authority: identity.authority,
            marker: marker.clone(),
            version: marker,
            channel,
            commit,
            tree,
            configuration: identity.configuration,
            profile: identity.profile,
            spec: identity.spec,
            state: "locked",
            datum,
            promotion,
        })
    }
    fn identity(&self, marker: &str) -> Result<Identity, String> {
        let reference = format!("refs/tags/{marker}");
        let kind = self.read(["cat-file", "-t", &reference])?;
        if kind != "tag" {
            return Err(format!("release marker {marker} is not an annotated tag"));
        }
        let message = self.read(["for-each-ref", "--format=%(contents)", &reference])?;
        if let Some(identity) = binding::resolve(message.trim(), &self.root, marker)? {
            return Ok(identity);
        }
        let spec = Spec::controller(&self.root)?;
        let product = spec.product.clone();
        let authority = spec.authority.clone();
        let wanted = format!("{product} {marker}");
        if message.trim() != wanted {
            return Err(format!(
                "release marker {marker} annotation disagrees with {wanted:?}"
            ));
        }
        Ok(Identity {
            schema: "plumb.release-marker/v1",
            product,
            authority,
            configuration: None,
            profile: None,
            datum: None,
            spec: Box::new(spec),
        })
    }

    fn stood(&self, marker: &str, line: &str, commit: &str) -> Result<(), String> {
        let carried = command(&self.root, ["merge-base", "--is-ancestor", commit, line])?;
        if carried.status.success() {
            return Ok(());
        }
        let head = self.read(["rev-parse", line])?;
        Err(format!(
            "release marker {marker} at {commit} is not carried by {line} at {head}"
        ))
    }

    fn promotion(
        &self,
        product: &str,
        authority: &str,
        version: &str,
        commit: &str,
    ) -> Result<Exact, String> {
        let exact = promotion::derive(&self.root, authority, commit, version)?;
        let url = format!(
            "{}/v1/releases/{}/{}/seal.json",
            authority, exact.channel, exact.version
        );
        let (seal, digest) = verify::Surface(&url).sealed(false)?;
        let standing = (
            seal.product.as_str(),
            seal.commit.as_str(),
            &seal.channel,
            &seal.version,
        );
        let wanted = (product, commit, &exact.channel, &exact.version);
        if standing != wanted {
            return Err(format!(
                "promotion seal does not prove release marker {}",
                exact.version
            ));
        }
        Ok(Exact {
            marker: exact.version,
            channel: exact.channel,
            seal: Seal {
                url,
                sha256: digest,
            },
        })
    }
}

pub(in crate::command) fn annotation(
    spec: &Spec,
    marker: &str,
    head: &str,
) -> Result<String, String> {
    binding::annotation(spec, marker, head)
}
fn guarded(product: &str, marker: &str) -> bool {
    product == "plumb"
        && marker
            .trim_start_matches('v')
            .parse::<semver::Version>()
            .is_ok_and(|version| (version.major, version.minor, version.patch) >= (0, 37, 8))
}

fn named(raw: &str) -> String {
    if raw.starts_with('v') {
        raw.to_string()
    } else {
        format!("v{raw}")
    }
}
