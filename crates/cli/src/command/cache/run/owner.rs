use super::record::{self, Record};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

pub(super) struct Scope {
    pub directory: PathBuf,
    pub registry: PathBuf,
    record: Record,
}

impl Scope {
    pub fn new(home: &Path) -> Result<Self, String> {
        let root = home.join("tmp");
        let registry = home.join("state").join("cache").join("run");
        std::fs::create_dir_all(&root).map_err(|error| error.to_string())?;
        std::fs::create_dir_all(&registry).map_err(|error| error.to_string())?;
        let root = root.canonicalize().map_err(|error| error.to_string())?;
        let registry = registry.canonicalize().map_err(|error| error.to_string())?;
        let directory = tempfile::Builder::new()
            .prefix("run-")
            .tempdir_in(&root)
            .map_err(|error| error.to_string())?;
        let path = directory
            .path()
            .canonicalize()
            .map_err(|error| error.to_string())?;
        let metadata = std::fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
        let record = Record {
            schema: "plumb.cache-run-owned/v1".into(),
            keeper: "plumb".into(),
            directory: path.clone(),
            device: metadata.dev(),
            inode: metadata.ino(),
        };
        let name = path.file_name().ok_or("compilation scope has no name")?;
        let registry = registry.join(name).with_extension("json");
        record::write(&path.join("plumb.json"), &record)?;
        record::write(&registry, &record)?;
        let _retained = directory.keep();
        let directory = path;
        Ok(Self {
            directory,
            registry,
            record,
        })
    }

    pub fn verify(&self) -> Result<(), String> {
        let metadata =
            std::fs::symlink_metadata(&self.directory).map_err(|error| error.to_string())?;
        if !metadata.is_dir()
            || metadata.dev() != self.record.device
            || metadata.ino() != self.record.inode
        {
            return Err("compilation scope directory ownership changed; preserve it".into());
        }
        if record::read(&self.registry)? != self.record
            || record::read(&self.directory.join("plumb.json"))? != self.record
        {
            return Err("compilation scope ownership records changed; preserve them".into());
        }
        Ok(())
    }

    pub fn remove(&self) -> Result<(), String> {
        self.verify()?;
        std::fs::remove_dir_all(&self.directory).map_err(|error| error.to_string())?;
        std::fs::remove_file(&self.registry).map_err(|error| error.to_string())
    }
}
