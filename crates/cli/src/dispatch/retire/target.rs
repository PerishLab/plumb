use super::super::release::model::Spec;
use plumb::forgejo::{Remote, git};
use std::path::{Path, PathBuf};

pub struct Target {
    pub root: PathBuf,
    pub product: String,
    pub bucket: String,
    pub domain: String,
    pub zone: String,
    pub remote: Remote,
}

impl Target {
    pub fn read(root: &Path) -> Result<Self, String> {
        let spec = Spec::read(&root.join("plumb.toml"))?;
        let retire = spec
            .retire
            .ok_or_else(|| "plumb.toml declares no release.retire seat".to_string())?;
        let domain = host(&spec.authority)?;
        Ok(Self {
            root: root.to_path_buf(),
            product: spec.product,
            bucket: retire.bucket,
            domain,
            zone: retire.zone,
            remote: git::remote(root, "")?,
        })
    }

    pub fn repo(&self) -> String {
        format!("{}/{}", self.remote.owner, self.remote.repo)
    }

    pub fn escrow(&self) -> PathBuf {
        self.root.join(".forgejo").join("release.env")
    }

    pub fn confirm(&self, held: &super::Deed) -> Result<(), String> {
        for (name, seen, wanted) in [
            ("repo", &held.repo, &self.repo()),
            ("bucket", &held.bucket, &self.bucket),
            ("domain", &held.domain, &self.domain),
        ] {
            if seen != wanted {
                return Err(format!("--confirm-{name} must exactly equal {wanted}"));
            }
        }
        Ok(())
    }
}

fn host(authority: &str) -> Result<String, String> {
    let rest = authority
        .strip_prefix("https://")
        .ok_or_else(|| format!("release authority is not an https URL: {authority}"))?;
    let held = rest.split('/').next().unwrap_or_default();
    if held.is_empty() || !held.contains('.') {
        return Err(format!("release authority has no hostname: {authority}"));
    }
    Ok(held.to_string())
}
