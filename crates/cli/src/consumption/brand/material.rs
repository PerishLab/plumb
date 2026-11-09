use plumb::packages::{Archive, Origin, Resolution};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Materials {
    pub(super) origins: Vec<Origin>,
    pub(super) publications: Vec<Publication>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct Publication {
    authority: String,
    marker: String,
    source: String,
    run: Option<String>,
}

impl Materials {
    pub(super) fn capture(
        root: &Path,
        resolution: &Resolution,
        design: bool,
    ) -> Result<Self, String> {
        let mut origins = Vec::new();
        let mut publications = Vec::new();
        let names = if design {
            vec!["@perishlab/crest", "@perishlab/design"]
        } else {
            vec!["@perishlab/crest"]
        };
        for name in names {
            let packages = resolution
                .packages
                .iter()
                .filter(|package| package.ecosystem == "npm" && package.name == name)
                .collect::<Vec<_>>();
            if packages.len() != 1 {
                return Err(format!(
                    "brand needs one exact formal resolution for {name}"
                ));
            }
            let package = packages[0];
            let origin = Origin::capture(root, package)?;
            locked(root, &origin)?;
            let product = if name == "@perishlab/crest" {
                "crest"
            } else {
                "design"
            };
            let authority = plumb::seat::release::Authority::at(&format!(
                "https://releases.{product}.perish.uk"
            ));
            let marker = plumb::seat::release::Marker {
                channel: "stable".into(),
                marker: format!("v{}", package.version),
            };
            let distribution = authority
                .read(&marker)?
                .ok_or("brand source has no formal distribution")?;
            if !distribution.complete()
                || distribution.media.get("npm").map(String::as_str) != Some("published")
            {
                return Err(format!("{name} has no complete formal npm distribution"));
            }
            if distribution.commit.len() != 40
                || !distribution
                    .commit
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit())
            {
                return Err("formal brand distribution has no qualified source identity".into());
            }
            publications.push(Publication {
                authority: authority.base().into(),
                marker: distribution.marker,
                source: distribution.commit,
                run: distribution.run,
            });
            origins.push(origin);
        }
        Ok(Self {
            origins,
            publications,
        })
    }

    pub(super) fn archive(&self, name: &str) -> Result<Archive, String> {
        self.origins
            .iter()
            .find(|origin| origin.package.name == name)
            .ok_or_else(|| format!("missing captured origin {name}"))?
            .read()
    }

    pub(super) fn origin(&self, name: &str) -> Result<&Origin, String> {
        self.origins
            .iter()
            .find(|origin| origin.package.name == name)
            .ok_or_else(|| format!("missing captured origin {name}"))
    }

    pub(super) fn installed(&self, app: &Path) -> Result<(), String> {
        let design = package(app, "@perishlab/design")?;
        self.compare("@perishlab/design", &design)?;
        let crest =
            package(&design, "@perishlab/crest").or_else(|_| package(app, "@perishlab/crest"))?;
        self.compare("@perishlab/crest", &crest)?;
        Ok(())
    }

    fn compare(&self, name: &str, directory: &Path) -> Result<(), String> {
        for (path, bytes) in self.archive(name)?.files {
            let actual = regular(directory, &path)?;
            if actual != bytes {
                return Err(format!(
                    "installed {name}/{path} differs from its formal archive"
                ));
            }
        }
        Ok(())
    }
}

fn locked(root: &Path, origin: &Origin) -> Result<(), String> {
    let bytes = std::fs::read(root.join("pnpm-lock.yaml")).map_err(|error| error.to_string())?;
    let lock: serde_yaml_ng::Value =
        serde_yaml_ng::from_slice(&bytes).map_err(|error| error.to_string())?;
    if lock["lockfileVersion"].as_str() != Some("9.0") {
        return Err("brand needs qualified pnpm lock format 9.0".into());
    }
    let key = format!("{}@{}", origin.package.name, origin.package.version);
    if lock["packages"][&key]["resolution"]["integrity"].as_str() != Some(&origin.integrity) {
        return Err(format!(
            "{key} lock integrity differs from the fixed formal registry"
        ));
    }
    Ok(())
}

fn package(from: &Path, name: &str) -> Result<PathBuf, String> {
    for parent in from.ancestors() {
        let path = parent.join("node_modules").join(name);
        if path.is_dir() {
            return path.canonicalize().map_err(|error| error.to_string());
        }
    }
    Err(format!(
        "no installed {name} resolves from {}",
        from.display()
    ))
}

pub(super) fn regular(root: &Path, path: &str) -> Result<Vec<u8>, String> {
    super::source::relative(path)?;
    let file = root.join(path);
    let metadata = std::fs::symlink_metadata(&file).map_err(|error| error.to_string())?;
    if !metadata.is_file() || metadata.len() > 16 * 1024 * 1024 {
        return Err(format!(
            "brand material {path} is not a bounded regular file"
        ));
    }
    let base = root.canonicalize().map_err(|error| error.to_string())?;
    if !file
        .canonicalize()
        .map_err(|error| error.to_string())?
        .starts_with(base)
    {
        return Err("brand material escapes its selected root".into());
    }
    std::fs::read(file).map_err(|error| error.to_string())
}

pub(super) fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub(super) fn unique<T>(entries: Vec<(String, T)>) -> Result<BTreeMap<String, T>, String> {
    let mut held = BTreeMap::new();
    for (key, value) in entries {
        if held.insert(key, value).is_some() {
            return Err("duplicate brand asset identity".into());
        }
    }
    Ok(held)
}
