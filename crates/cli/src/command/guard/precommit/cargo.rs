use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

pub(super) struct Lease {
    pub root: PathBuf,
    _lock: File,
}

impl Lease {
    pub fn new(root: &Path) -> Result<Self, String> {
        let root = seat(root)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root.join("lease"))
            .map_err(|error| format!("cannot open Guard compilation lease: {error}"))?;
        lock.lock()
            .map_err(|error| format!("cannot hold Guard compilation lease: {error}"))?;
        Ok(Self { root, _lock: lock })
    }
}

fn seat(root: &Path) -> Result<PathBuf, String> {
    let common = super::tree::git(
        root,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
        "resolve the guarded repository",
    )?;
    let common = std::fs::canonicalize(&common)
        .map_err(|error| format!("cannot resolve guarded repository {common}: {error}"))?;
    let identity = common
        .to_str()
        .ok_or_else(|| "guarded repository path is not UTF-8".to_string())?;
    let home = plumb::config::value("PLUMB_HOME")
        .map(PathBuf::from)
        .or_else(|| plumb::config::data("plumb"))
        .ok_or_else(|| "cannot cache Guard compilation: no PLUMB_HOME".to_string())?;
    let seat = cache(&home, identity);
    std::fs::create_dir_all(&seat)
        .map_err(|error| format!("cannot create {}: {error}", seat.display()))?;
    Ok(seat)
}

fn cache(home: &Path, identity: &str) -> PathBuf {
    let digest = plumb::depot::sha(identity.as_bytes());
    #[cfg(windows)]
    {
        home.join("c").join(&digest[..32])
    }
    #[cfg(not(windows))]
    {
        home.join("cache").join("guard").join("cargo").join(digest)
    }
}

#[cfg(test)]
mod tests {
    use super::cache;
    use std::path::Path;

    #[test]
    fn bounded() {
        let home = Path::new("C:/Users/operator/AppData/Local/plumb");
        let first = cache(home, &"repository/".repeat(100));
        let second = cache(home, &"repository/".repeat(99));
        assert_ne!(first, second);
        #[cfg(windows)]
        {
            assert_eq!(first.parent(), Some(home.join("c").as_path()));
            assert_eq!(first.file_name().unwrap().len(), 32);
            assert_eq!(first.as_os_str().len(), home.as_os_str().len() + 35);
        }
    }
}
