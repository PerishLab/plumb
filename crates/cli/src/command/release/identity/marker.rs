use super::super::truth::{record, verify};
use super::promotion;
use crate::command::release::{Marker as Deed, channel};
use crate::shape::release::Spec;
use plumb::datum;
use plumb::forgejo::git;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const SCHEMA: &str = "plumb.release-marker/v1";

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
    state: &'static str,
    datum: Datum,
    #[serde(skip_serializing_if = "Option::is_none")]
    promotion: Option<Exact>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Datum {
    path: String,
    sha256: String,
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

struct Seat {
    root: PathBuf,
    product: String,
    authority: String,
    repository: String,
}

pub fn run(deed: Deed) -> Result<String, String> {
    let (name, show, held) = match deed {
        Deed::Show { marker, held } => (marker, true, held),
        Deed::Verify { marker, held } => (marker, false, held),
    };
    let marker = resolve(&name, !held)?;
    if show {
        let mut text = serde_json::to_string_pretty(&marker).map_err(|error| error.to_string())?;
        text.push('\n');
        Ok(text)
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
    Seat::open()?.resolve(raw, refresh)
}

pub(in crate::command) fn marked(
    root: &Path,
    product: &str,
    authority: &str,
    raw: &str,
) -> Result<Descriptor, String> {
    Seat::new(
        root.to_path_buf(),
        product.to_string(),
        authority.to_string(),
    )?
    .resolve(raw, true)
}

impl Descriptor {
    pub(in crate::command) fn digest(&self) -> Result<String, String> {
        serde_json::to_vec(self)
            .map(|bytes| record::sha(&bytes))
            .map_err(|error| error.to_string())
    }
}

impl Seat {
    fn open() -> Result<Self, String> {
        let root = git::root()?;
        let spec = Spec::read(&root.join("plumb.toml"))?;
        Self::new(root, spec.product, spec.authority)
    }

    fn new(root: PathBuf, product: String, authority: String) -> Result<Self, String> {
        let remote = git::remote(&root, "")?;
        Ok(Self {
            root,
            product,
            authority,
            repository: format!("{}/{}", remote.owner, remote.repo),
        })
    }

    fn resolve(&self, raw: &str, refresh: bool) -> Result<Descriptor, String> {
        let marker = named(raw);
        let channel = channel::channel(&marker)
            .map_err(|error| format!("invalid release marker {marker}: {error}"))?;
        refresh.then(|| self.fetch()).transpose()?;
        self.annotated(&marker)?;
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
            Err(error) if guarded(&self.product, &marker) => {
                return Err(format!(
                    "release marker {marker} has no valid guard proof: {error}"
                ));
            }
            Err(_) => {}
        }
        let version = marker.split('-').next().unwrap_or(&marker).to_string();
        self.stood(&marker, &version, &commit)?;
        let datum = self.datum(&version, &commit)?;
        let promotion = if channel == "stable" {
            Some(self.promotion(&version, &commit)?)
        } else {
            None
        };
        Ok(Descriptor {
            schema: SCHEMA,
            product: self.product.clone(),
            repository: self.repository.clone(),
            authority: self.authority.clone(),
            marker: marker.clone(),
            version: marker,
            channel,
            commit,
            tree,
            state: "locked",
            datum,
            promotion,
        })
    }
    fn fetch(&self) -> Result<(), String> {
        git::fetch(&self.root)?;
        success(
            "fetch release markers",
            Command::new("git")
                .args(["fetch", "--tags", "origin"])
                .current_dir(&self.root)
                .output(),
        )
        .map(|_| ())
    }
    fn annotated(&self, marker: &str) -> Result<(), String> {
        let reference = format!("refs/tags/{marker}");
        let kind = self.read(["cat-file", "-t", &reference])?;
        if kind != "tag" {
            return Err(format!("release marker {marker} is not an annotated tag"));
        }
        let message = self.read(["for-each-ref", "--format=%(contents)", &reference])?;
        let wanted = format!("{} {marker}", self.product);
        if message.trim() != wanted {
            return Err(format!(
                "release marker {marker} annotation disagrees with {wanted:?}"
            ));
        }
        Ok(())
    }

    fn stood(&self, marker: &str, version: &str, commit: &str) -> Result<(), String> {
        let line = format!("origin/release/{version}");
        let head = self.read(["rev-parse", &line])?;
        if head == commit {
            Ok(())
        } else {
            Err(format!(
                "release marker {marker} stands at {commit}, not {line} at {head}"
            ))
        }
    }

    fn datum(&self, version: &str, commit: &str) -> Result<Datum, String> {
        let path = datum::leaf(version);
        let object = format!("{commit}:{path}");
        let bytes = self.bytes(["show", &object])?;
        datum::decode(version, &bytes)?;
        Ok(Datum {
            path,
            sha256: record::sha(&bytes),
        })
    }

    fn promotion(&self, version: &str, commit: &str) -> Result<Exact, String> {
        let exact = promotion::derive(&self.root, &self.authority, commit, version)?;
        let url = format!(
            "{}/v1/releases/{}/{}/seal.json",
            self.authority, exact.channel, exact.version
        );
        let (seal, digest) = verify::Surface(&url).sealed(false)?;
        let standing = (
            &seal.product,
            seal.commit.as_str(),
            &seal.channel,
            &seal.version,
        );
        let wanted = (&self.product, commit, &exact.channel, &exact.version);
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

    fn read<const N: usize>(&self, args: [&str; N]) -> Result<String, String> {
        let output = command(&self.root, args)?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    fn bytes<const N: usize>(&self, args: [&str; N]) -> Result<Vec<u8>, String> {
        let output = command(&self.root, args)?;
        if output.status.success() {
            Ok(output.stdout)
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }
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

fn command<const N: usize>(root: &Path, args: [&str; N]) -> Result<Output, String> {
    Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| format!("cannot run git: {error}"))
}

fn success(action: &str, output: std::io::Result<Output>) -> Result<String, String> {
    let output = output.map_err(|error| format!("cannot run git: {error}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(format!(
            "cannot {action}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}
