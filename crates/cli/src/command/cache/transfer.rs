use super::parcel::{Parcel, Plan, Receipt, Snapshot};
use super::transport::{
    Context, MARKER,
    record::Hint,
    target::{NAMES, Target},
};
use crate::command::guard::precommit::cargo::Lease;
use std::path::{Path, PathBuf};

pub(super) enum Action {
    Inspect,
    Restore { fetch: bool },
    Save,
}

pub(super) struct Transfer<'a> {
    pub root: &'a Path,
    pub home: &'a Path,
    pub context: Option<&'a Context>,
    pub parcel: &'a Path,
    pub identity: &'a str,
}

impl Transfer<'_> {
    pub fn run(&self, action: Action) -> Result<Receipt, String> {
        let parent = self
            .parcel
            .parent()
            .ok_or("compilation parcel has no parent")?;
        if let Ok(metadata) = std::fs::symlink_metadata(parent)
            && (!metadata.is_dir() || metadata.file_type().is_symlink())
        {
            return Err("compilation transport refuses a foreign slot".into());
        }
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        let lease = super::lease(parent)?;
        lease.lock().map_err(|error| error.to_string())?;
        match action {
            Action::Inspect => Ok(Receipt::default()),
            Action::Save => self.save(),
            Action::Restore { fetch } => self.fetch(fetch),
        }
    }

    fn fetch(&self, fetch: bool) -> Result<Receipt, String> {
        let receipt = self.restore()?;
        if receipt.cargo || !fetch {
            return Ok(receipt);
        }
        match super::transport::provider::fetch(self.root, self.parcel, self.identity) {
            Ok(true) => self.restore(),
            Ok(false) => Ok(receipt),
            Err(error) => {
                eprintln!("guard material: heat source unavailable; compiling normally: {error}");
                Ok(receipt)
            }
        }
    }

    fn save(&self) -> Result<Receipt, String> {
        let lease = Lease::new(self.root)?;
        let cargo = match self.context {
            Some(context) if Hint::ready(&lease.root, context)? => {
                (Target(&lease.root).seal()?.links == 0).then(|| lease.root.clone())
            }
            _ => None,
        };
        let compiler = self.home.join("cache/compiler");
        let compiler =
            (compiler.is_dir() && std::env::var_os("SCCACHE_DIR").is_none()).then_some(compiler);
        Parcel(self.parcel).write(Plan {
            identity: self.identity.into(),
            cargo,
            compiler,
        })
    }

    fn restore(&self) -> Result<Receipt, String> {
        let snapshot = match Parcel(self.parcel).read(self.identity) {
            Ok(Some(snapshot)) => snapshot,
            Ok(None) => return Ok(Receipt::default()),
            Err(error) => {
                eprintln!("guard material: rejected parcel; compiling normally: {error}");
                return Ok(Receipt::default());
            }
        };
        let lease = Lease::new(self.root)?;
        let cargo = self.native(&snapshot, &lease)?;
        let compiler = if snapshot.manifest.compiler {
            Copy {
                source: snapshot.root.path().join("compiler"),
                target: self.home.join("cache/compiler"),
            }
            .merge()?
                != 0
        } else {
            false
        };
        Ok(Receipt { cargo, compiler })
    }

    fn native(&self, snapshot: &Snapshot, lease: &Lease) -> Result<bool, String> {
        let Some(context) = self.context else {
            return Ok(false);
        };
        if !snapshot.manifest.cargo {
            return Ok(false);
        }
        let source = snapshot.root.path().join("cargo");
        let valid = Hint::ready(&source, context).unwrap_or(false);
        if !valid {
            eprintln!("guard material: rejected native Cargo snapshot; compiling normally");
            return Ok(false);
        }
        if !Target(&lease.root).supported()? {
            return Err("native Cargo transport refuses an unsupported existing cache".into());
        }
        Hint::read(&lease.root)?;
        plumb::replace::write(&lease.root.join(MARKER), b"{\"ready\":false}")
            .map_err(|error| error.to_string())?;
        Target(&lease.root).reset()?;
        for name in NAMES.into_iter().chain([MARKER]) {
            let path = source.join(name);
            if path.exists() {
                let target = lease.root.join(name);
                if target.exists() {
                    std::fs::remove_file(&target).map_err(|error| error.to_string())?;
                }
                std::fs::rename(&path, target).map_err(|error| error.to_string())?;
            }
        }
        Ok(true)
    }
}

struct Copy {
    source: PathBuf,
    target: PathBuf,
}

impl Copy {
    fn merge(&self) -> Result<u64, String> {
        if let Ok(metadata) = std::fs::symlink_metadata(&self.target)
            && (!metadata.is_dir() || metadata.file_type().is_symlink())
        {
            return Err("compiler transport refuses a foreign directory".into());
        }
        std::fs::create_dir_all(&self.target).map_err(|error| error.to_string())?;
        let mut files = 0;
        for entry in std::fs::read_dir(&self.source).map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
            let source = entry.path();
            let target = self.target.join(entry.file_name());
            let metadata = entry.file_type().map_err(|error| error.to_string())?;
            if metadata.is_dir() {
                files += Copy { source, target }.merge()?;
            } else if metadata.is_file() {
                files += self.file(&source, &target)?;
            } else {
                return Err("compiler transport contains an unsupported file".into());
            }
        }
        Ok(files)
    }

    fn file(&self, source: &Path, target: &Path) -> Result<u64, String> {
        if let Ok(metadata) = std::fs::symlink_metadata(target) {
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err("compiler transport refuses a foreign entry".into());
            }
            return Ok(0);
        }
        match std::fs::hard_link(source, target) {
            Ok(()) => return Ok(1),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => return Ok(0),
            Err(_) => {}
        }
        let mut file =
            tempfile::NamedTempFile::new_in(&self.target).map_err(|error| error.to_string())?;
        let mut input = std::fs::File::open(source).map_err(|error| error.to_string())?;
        std::io::copy(&mut input, &mut file).map_err(|error| error.to_string())?;
        match file.persist_noclobber(target) {
            Ok(_) => Ok(1),
            Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => Ok(0),
            Err(error) => Err(error.error.to_string()),
        }
    }
}
