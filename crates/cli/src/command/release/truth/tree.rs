use std::path::{Path, PathBuf};
use std::process::Output;

pub struct Seat {
    repository: PathBuf,
    staged: tempfile::TempDir,
}

impl Seat {
    pub fn open(repository: &Path, commit: &str) -> Result<Self, String> {
        if !known(repository, commit)? {
            success(
                "fetch release commit",
                git(repository, ["fetch", "--depth=1", "origin", commit])?,
            )?;
        }
        let staged = tempfile::tempdir()
            .map_err(|error| format!("cannot stage the release source: {error}"))?;
        let path = staged
            .path()
            .to_str()
            .ok_or_else(|| "release source seat is not UTF-8".to_string())?;
        success(
            "stage release source",
            git(repository, ["worktree", "add", "--detach", path, commit])?,
        )?;
        Ok(Self {
            repository: repository.to_path_buf(),
            staged,
        })
    }

    pub fn path(&self) -> &Path {
        self.staged.path()
    }

    pub fn govern(&self, plan: &crate::shape::depot::Batch) -> Result<(), String> {
        let target = crate::shape::product::candidate(self.path(), plan)?;
        let Some(profile) = target.profile else {
            return Ok(());
        };
        for (name, content) in [
            ("plumb.toml", profile.manifest),
            ("ectropy.toml", profile.ectropy),
        ] {
            let path = self.path().join(name);
            match std::fs::symlink_metadata(&path) {
                Ok(held) if !held.is_file() => {
                    return Err(format!("candidate governance requires a regular {name}"));
                }
                Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                    return Err(format!("cannot inspect candidate {name}: {error}"));
                }
                _ => {}
            }
            std::fs::write(&path, content)
                .map_err(|error| format!("cannot project candidate {name}: {error}"))?;
        }
        crate::command::render::affirm::candidate::Review(self.path()).project(plan)
    }
}

impl Drop for Seat {
    fn drop(&mut self) {
        let Some(path) = self.staged.path().to_str() else {
            return;
        };
        let _ = git(&self.repository, ["worktree", "remove", "--force", path]);
    }
}

fn known(repository: &Path, commit: &str) -> Result<bool, String> {
    let object = format!("{commit}^{{commit}}");
    let output = git(repository, ["cat-file", "-e", &object])?;
    Ok(output.status.success())
}

fn git<const N: usize>(repository: &Path, args: [&str; N]) -> Result<Output, String> {
    plumb::config::detached("git")
        .args(args)
        .current_dir(repository)
        .output()
        .map_err(|error| format!("cannot run git: {error}"))
}

fn success(action: &str, output: Output) -> Result<(), String> {
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "cannot {action}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}
