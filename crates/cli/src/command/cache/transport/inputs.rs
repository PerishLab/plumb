use super::stamp::Stamp;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::collections::BTreeMap;
use std::io::Read as _;
use std::path::{Component, Path};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Input {
    pub sha: String,
    pub stamp: Stamp,
    pub mode: u32,
}

impl Input {
    fn read(path: &Path) -> Result<Option<Self>, String> {
        let metadata = std::fs::symlink_metadata(path)
            .map_err(|error| format!("cannot inspect Guard input {}: {error}", path.display()))?;
        if !metadata.is_file() {
            return Ok(None);
        }
        let sha = digest(path)?;
        Ok(Some(Self {
            sha,
            stamp: Stamp::at(metadata.modified().map_err(|error| error.to_string())?)?,
            mode: mode(&metadata),
        }))
    }

    pub fn valid(&self, until: Stamp) -> bool {
        super::context::hex(&self.sha) && self.stamp.time().is_ok() && self.stamp <= until
    }

    pub fn same(&self, other: &Self) -> bool {
        self.sha == other.sha && self.mode == other.mode
    }
}

pub(crate) struct Inputs<'a>(pub &'a Path);

impl Inputs<'_> {
    pub fn restore(
        &self,
        current: &BTreeMap<String, Input>,
        old: &BTreeMap<String, Input>,
        fresh: std::time::SystemTime,
    ) -> Result<(), String> {
        for (name, input) in current {
            let time = old
                .get(name)
                .filter(|old| input.same(old))
                .map(|old| old.stamp.time())
                .transpose()?
                .unwrap_or(fresh);
            let file = std::fs::File::options()
                .read(true)
                .write(cfg!(windows))
                .open(self.0.join(name))
                .map_err(|error| error.to_string())?;
            file.set_modified(time)
                .map_err(|error| format!("cannot restore verified Guard freshness: {error}"))?;
        }
        Ok(())
    }

    pub fn read(&self) -> Result<Option<BTreeMap<String, Input>>, String> {
        let output = plumb::config::detached("git")
            .arg("-C")
            .arg(self.0)
            .args(["ls-files", "--cached", "-z"])
            .output()
            .map_err(|error| error.to_string())?;
        if !output.status.success() {
            return Err("cannot enumerate exact Guard material inputs".into());
        }
        let mut files = BTreeMap::new();
        for path in output
            .stdout
            .split(|byte| *byte == 0)
            .filter(|path| !path.is_empty())
        {
            let name =
                std::str::from_utf8(path).map_err(|_| "Guard material input is not UTF-8")?;
            if !relative(name) {
                return Err("Guard material input is not a relative file".into());
            }
            let Some(input) = Input::read(&self.0.join(name))? else {
                return Ok(None);
            };
            files.insert(name.to_string(), input);
        }
        Ok(Some(files))
    }
}

pub(super) fn relative(name: &str) -> bool {
    !name.is_empty()
        && Path::new(name)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

pub(super) fn digest(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|error| {
        format!(
            "cannot read compilation material {}: {error}",
            path.display()
        )
    })?;
    let mut hash = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let count = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

#[cfg(unix)]
pub(crate) fn mode(metadata: &std::fs::Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt as _;
    metadata.permissions().mode() & 0o777
}

#[cfg(not(unix))]
pub(crate) fn mode(metadata: &std::fs::Metadata) -> u32 {
    u32::from(metadata.permissions().readonly())
}
