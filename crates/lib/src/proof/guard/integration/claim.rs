use super::owner::Owner;
use super::staging::{PREFIX, live, same};
use radicula_lock::Mode;
use std::fs::{File, FileTimes};
use std::path::{Path, PathBuf};

pub struct Claim {
    pub root: PathBuf,
    path: PathBuf,
    retained: bool,
    _lock: File,
    read: std::sync::Mutex<()>,
}

impl Claim {
    pub fn new(base: &Path, source: &Path) -> Result<Self, String> {
        loop {
            let (lock, path) = tempfile::Builder::new()
                .prefix(PREFIX)
                .suffix(".lock")
                .tempfile_in(base)
                .and_then(|file| file.keep().map_err(|error| error.error))
                .map_err(|error| format!("cannot reserve guard worktree: {error}"))?;
            radicula_lock::lock(&lock, Mode::Exclusive)
                .map_err(|error| format!("cannot hold guard worktree: {error}"))?;
            if same(&path, &lock) {
                return Ok(Self::held(source, path, lock, false));
            }
        }
    }

    pub fn retain(base: &Path, source: &Path) -> Result<Self, String> {
        let base = dunce::canonicalize(base)
            .map_err(|error| format!("cannot resolve Guard source home: {error}"))?;
        let owner = Owner::new(source)?;
        let root = owner.path(&base)?;
        let path = root.with_extension("lock");
        let (lock, created) = reserve(&path)?;
        radicula_lock::lock(&lock, Mode::Exclusive)
            .map_err(|error| format!("cannot hold retained Guard source: {error}"))?;
        if !same(&path, &lock) {
            return Err("Guard source owner moved while acquiring its lease".into());
        }
        if created {
            if root.try_exists().map_err(|error| error.to_string())? {
                let _ = std::fs::remove_file(&path);
                return Err("refusing an existing Guard source without its owner record".into());
            }
            owner.write(&lock)?;
        } else if Owner::load(&lock)? != owner {
            return Err("retained Guard source ownership disagrees with its repository".into());
        }
        owner.verify(&root)?;
        touch(&lock);
        Ok(Self::held(source, path, lock, true))
    }

    pub fn verify(&self) -> bool {
        let Ok(_read) = self.read.lock() else {
            return false;
        };
        self.path
            .symlink_metadata()
            .is_ok_and(|metadata| metadata.is_file())
            && same(&self.path, &self._lock)
            && Owner::load(&self._lock).is_ok_and(|owner| owner.owns(&self.root))
    }

    fn held(source: &Path, path: PathBuf, lock: File, retained: bool) -> Self {
        let root = path.with_extension("");
        live().push((source.to_path_buf(), root.clone()));
        Self {
            root,
            path,
            retained,
            _lock: lock,
            read: std::sync::Mutex::new(()),
        }
    }
}

impl Drop for Claim {
    fn drop(&mut self) {
        if self.retained {
            touch(&self._lock);
        } else {
            let _ = std::fs::remove_file(&self.path);
        }
        live().retain(|(_, root)| root != &self.root);
        let _ = radicula_lock::unlock(&self._lock);
    }
}

fn reserve(path: &Path) -> Result<(File, bool), String> {
    match File::options()
        .read(true)
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(file) => Ok((file, true)),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let metadata = std::fs::symlink_metadata(path).map_err(|error| error.to_string())?;
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err("Guard source owner must be a regular file".into());
            }
            File::options()
                .read(true)
                .write(true)
                .open(path)
                .map(|file| (file, false))
                .map_err(|error| error.to_string())
        }
        Err(error) => Err(format!("cannot reserve retained Guard source: {error}")),
    }
}

fn touch(file: &File) {
    let _ = file.set_times(FileTimes::new().set_modified(std::time::SystemTime::now()));
}

#[cfg(test)]
mod tests {
    use super::Claim;

    #[test]
    fn reads() {
        let repository = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let output = crate::config::detached("git")
            .arg("-C")
            .arg(repository.path())
            .args(["init", "--quiet"])
            .output()
            .unwrap();
        assert!(output.status.success());
        let claim = std::sync::Arc::new(Claim::retain(home.path(), repository.path()).unwrap());
        let threads = (0..8)
            .map(|_| {
                let claim = claim.clone();
                std::thread::spawn(move || {
                    for _ in 0..16 {
                        assert!(
                            claim.verify(),
                            "concurrent owner reads keep their complete record"
                        );
                    }
                })
            })
            .collect::<Vec<_>>();
        for thread in threads {
            thread.join().unwrap();
        }
    }

    #[test]
    fn release() {
        let repository = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let output = crate::config::detached("git")
            .arg("-C")
            .arg(repository.path())
            .args(["init", "--quiet"])
            .output()
            .unwrap();
        assert!(output.status.success());
        let claim = Claim::retain(home.path(), repository.path()).unwrap();
        let inherited = claim._lock.try_clone().unwrap();
        let path = claim.root.with_extension("lock");
        drop(claim);
        let peer = std::fs::File::options()
            .read(true)
            .write(true)
            .open(path)
            .unwrap();
        assert!(
            radicula_lock::attempt(&peer, radicula_lock::Mode::Exclusive).is_ok(),
            "a copied descriptor must not retain a released source lease"
        );
        drop(inherited);
    }
}
