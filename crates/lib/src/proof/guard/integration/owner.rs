use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Seek;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Owner {
    schema: String,
    common: PathBuf,
}

impl Owner {
    pub fn retained(tree: &Path) -> bool {
        tree.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(super::staging::RETAINED))
    }

    pub fn new(source: &Path) -> Result<Self, String> {
        Ok(Self {
            schema: "plumb.guard-source/v1".into(),
            common: common(source)?,
        })
    }

    pub fn path(&self, base: &Path) -> Result<PathBuf, String> {
        let identity = self
            .common
            .to_str()
            .ok_or("Guard source identity is not UTF-8")?;
        let digest = crate::depot::sha(identity.as_bytes());
        Ok(base.join(format!(
            "{}{prefix}",
            super::staging::RETAINED,
            prefix = &digest[..32]
        )))
    }

    pub fn read(path: &Path) -> Result<Self, String> {
        let metadata = std::fs::symlink_metadata(path).map_err(|error| error.to_string())?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err("Guard source owner must be a regular file".into());
        }
        let file =
            File::open(path).map_err(|error| format!("cannot read Guard source owner: {error}"))?;
        Self::load(&file)
    }

    pub fn load(mut file: &File) -> Result<Self, String> {
        file.rewind()
            .map_err(|error| format!("cannot rewind Guard source owner: {error}"))?;
        let owner: Self = serde_json::from_reader(file)
            .map_err(|error| format!("cannot decode Guard source owner: {error}"))?;
        if owner.schema != "plumb.guard-source/v1" {
            return Err(format!("unknown Guard source schema {}", owner.schema));
        }
        Ok(owner)
    }

    pub fn write(&self, file: &File) -> Result<(), String> {
        serde_json::to_writer(file, self)
            .map_err(|error| format!("cannot record Guard source owner: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("cannot persist Guard source owner: {error}"))
    }

    pub fn verify(&self, root: &Path) -> Result<(), String> {
        match std::fs::symlink_metadata(root) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => (),
            _ => return Err("retained Guard source is not an owned directory".into()),
        }
        if common(root)? != self.common {
            return Err("retained Guard source belongs to another Git repository".into());
        }
        let link = root.join(".git");
        let metadata = std::fs::symlink_metadata(&link).map_err(|error| error.to_string())?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err("retained Guard source is not a linked worktree".into());
        }
        let admin = git(root, &["rev-parse", "--absolute-git-dir"])?;
        let admin = Path::new(&admin)
            .canonicalize()
            .map_err(|error| error.to_string())?;
        if admin.parent() != Some(self.common.join("worktrees").as_path()) {
            return Err("retained Guard source has foreign worktree administration".into());
        }
        let back =
            std::fs::read_to_string(admin.join("gitdir")).map_err(|error| error.to_string())?;
        if Path::new(back.trim()).canonicalize().ok() != link.canonicalize().ok() {
            return Err("retained Guard source registration disagrees with its owner".into());
        }
        let output = crate::config::detached("git")
            .arg("-C")
            .arg(root)
            .args(["symbolic-ref", "-q", "HEAD"])
            .output()
            .map_err(|error| format!("cannot inspect Guard source HEAD: {error}"))?;
        if output.status.code() != Some(1) {
            return Err("retained Guard source must have a detached HEAD".into());
        }
        Ok(())
    }

    pub fn owns(&self, root: &Path) -> bool {
        root.parent()
            .is_some_and(|base| self.path(base).is_ok_and(|path| path == root))
            && self.verify(root).is_ok()
    }

    pub fn orphaned(&self, root: &Path) -> bool {
        if !std::fs::symlink_metadata(&self.common)
            .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
        {
            return false;
        }
        if !root
            .parent()
            .is_some_and(|base| self.path(base).is_ok_and(|path| path == root))
        {
            return false;
        }
        if !std::fs::symlink_metadata(root)
            .is_ok_and(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink())
        {
            return false;
        }
        let link = root.join(".git");
        if !std::fs::symlink_metadata(&link)
            .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
        {
            return false;
        }
        std::fs::read_to_string(link).is_ok_and(|link| {
            link.trim()
                .strip_prefix("gitdir:")
                .map(|admin| Path::new(admin.trim()))
                .is_some_and(|admin| {
                    admin.parent() == Some(self.common.join("worktrees").as_path())
                })
        })
    }

    pub fn matches(&self, root: &Path) -> bool {
        if !self.owns(root) {
            return false;
        }
        if !root.exists() {
            return true;
        }
        git(root, &["status", "--porcelain", "--untracked-files=no"])
            .is_ok_and(|status| status.is_empty())
    }
}

fn common(root: &Path) -> Result<PathBuf, String> {
    let common = git(
        root,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?;
    Path::new(&common)
        .canonicalize()
        .map_err(|error| format!("cannot canonicalize Guard source repository: {error}"))
}

fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let output = crate::config::detached("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|error| format!("cannot inspect Guard source repository: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cannot inspect Guard source repository: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
