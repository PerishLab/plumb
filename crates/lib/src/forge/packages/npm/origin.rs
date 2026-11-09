use super::archive;

use super::{Package, process};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Origin {
    pub package: Package,
    pub integrity: String,
    pub archive: PathBuf,
}

pub struct Archive {
    pub files: BTreeMap<String, Vec<u8>>,
}

impl Origin {
    pub fn capture(root: &Path, package: &Package) -> Result<Self, String> {
        if package.ecosystem != "npm"
            || !package.name.starts_with("@perishlab/")
            || package.name.trim_start_matches("@perishlab/").contains('/')
        {
            return Err("origin capture requires a first-party npm package".into());
        }
        let version =
            semver::Version::parse(&package.version).map_err(|error| error.to_string())?;
        if !version.pre.is_empty() || version.to_string() != package.version {
            return Err("origin capture requires a canonical stable version".into());
        }
        let spec = format!("{}@{}", package.name, package.version);
        let metadata = process::run(
            root,
            "packages",
            &[
                "npm".into(),
                "view".into(),
                spec,
                "--json".into(),
                "--registry=https://npm.pkg.github.com/".into(),
                "--@perishlab:registry=https://npm.pkg.github.com/".into(),
            ],
        )?;
        let metadata: serde_json::Value = serde_json::from_slice(&metadata)
            .map_err(|error| format!("npm origin metadata is unreadable: {error}"))?;
        if metadata["name"] != package.name || metadata["version"] != package.version {
            return Err("npm origin metadata names a different package".into());
        }
        let integrity = metadata["dist"]["integrity"]
            .as_str()
            .ok_or("npm origin has no integrity")?;
        let url = metadata["dist"]["tarball"]
            .as_str()
            .ok_or("npm origin has no archive")?;
        let prefix = format!(
            "https://npm.pkg.github.com/download/{}/{}/",
            package.name, package.version
        );
        let tail = url
            .strip_prefix(&prefix)
            .ok_or("npm archive is outside its fixed package authority")?;
        if tail.is_empty() || !tail.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err("unsupported npm archive reference".into());
        }
        let home = crate::config::value("PLUMB_HOME")
            .map(PathBuf::from)
            .or_else(|| crate::config::data("plumb"))
            .ok_or("origin capture requires PLUMB_HOME")?;
        let cache = home.join("cache/packages/origin");
        std::fs::create_dir_all(&cache).map_err(|error| error.to_string())?;
        let digest = archive::identity(integrity)?;
        let archive = cache.join(format!("{digest}.tgz"));
        let held = Self {
            package: package.clone(),
            integrity: integrity.into(),
            archive,
        };
        match std::fs::symlink_metadata(&held.archive) {
            Ok(_) => {
                held.read()?;
                return Ok(held);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
        }
        let staging = tempfile::tempdir().map_err(|error| error.to_string())?;
        let bytes = process::run(
            staging.path(),
            "packages",
            &[
                "npm".into(),
                "pack".into(),
                url.into(),
                "--ignore-scripts".into(),
                "--json".into(),
                "--registry=https://npm.pkg.github.com/".into(),
                "--@perishlab:registry=https://npm.pkg.github.com/".into(),
            ],
        )?;
        let packed: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        let file = packed[0]["filename"]
            .as_str()
            .ok_or("npm pack omitted archive filename")?;
        if file.contains(['/', '\\']) || file == "." || file == ".." {
            return Err("npm pack emitted an unsafe archive filename".into());
        }
        let payload = archive::payload(&staging.path().join(file), integrity)?;
        let decoded = Archive::decode(&payload)?;
        decoded.identity(package)?;
        let mut output =
            tempfile::NamedTempFile::new_in(&cache).map_err(|error| error.to_string())?;
        use std::io::Write;
        output
            .write_all(&payload)
            .map_err(|error| error.to_string())?;
        if let Err(error) = output.persist_noclobber(&held.archive) {
            if error.error.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(error.to_string());
            }
            held.read()?;
        }
        Ok(held)
    }

    pub fn read(&self) -> Result<Archive, String> {
        let bytes = archive::payload(&self.archive, &self.integrity)?;
        let archive = Archive::decode(&bytes)?;
        archive.identity(&self.package)?;
        Ok(archive)
    }
}

impl Archive {
    fn identity(&self, package: &Package) -> Result<(), String> {
        let bytes = self
            .files
            .get("package.json")
            .ok_or("archive omits package.json")?;
        let manifest: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
        if manifest["name"] != package.name || manifest["version"] != package.version {
            return Err("archive package identity differs from its registry origin".into());
        }
        Ok(())
    }
}
