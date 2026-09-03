use std::path::{Path, PathBuf};
use std::process::Output;

pub(crate) struct Index {
    pub root: PathBuf,
    source: PathBuf,
}

impl Index {
    pub(super) fn new(source: &Path, tree: &str) -> Result<Self, String> {
        let parent = plumb::config::detached("git")
            .arg("-C")
            .arg(source)
            .args(["rev-parse", "--verify", "HEAD"])
            .output()
            .map_err(|error| format!("cannot run git to read HEAD: {error}"))?;
        let mut record = plumb::config::detached("git");
        record.arg("-C").arg(source).args(["commit-tree", tree]);
        if parent.status.success() {
            record
                .arg("-p")
                .arg(String::from_utf8_lossy(&parent.stdout).trim());
        }
        let output = record
            .args(["-m", "plumb staged guard"])
            .output()
            .map_err(|error| format!("cannot run git to record staged tree: {error}"))?;
        let commit = text(output, "record staged tree")?;
        let base = plumb::config::value("PLUMB_HOME")
            .map(PathBuf::from)
            .or_else(|| plumb::config::data("plumb"))
            .ok_or_else(|| "cannot stage guard worktree: no PLUMB_HOME".to_string())?
            .join("tmp");
        std::fs::create_dir_all(&base)
            .map_err(|error| format!("cannot create {}: {error}", base.display()))?;
        let temporary = tempfile::Builder::new()
            .prefix("guard-")
            .tempdir_in(&base)
            .map_err(|error| format!("cannot reserve guard worktree: {error}"))?;
        let root = temporary.path().to_path_buf();
        temporary
            .close()
            .map_err(|error| format!("cannot prepare guard worktree: {error}"))?;
        let output = plumb::config::detached("git")
            .arg("-C")
            .arg(source)
            .args(["worktree", "add", "--detach", "--quiet"])
            .arg(&root)
            .arg(&commit)
            .output()
            .map_err(|error| format!("cannot create guard worktree: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "cannot create guard worktree: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        Ok(Self {
            root,
            source: source.to_path_buf(),
        })
    }

    pub(crate) fn working(source: &Path) -> Result<Self, String> {
        let base = temporary()?;
        let index = base.path().join("index");
        let mut read = plumb::config::detached("git");
        let read = read
            .arg("-C")
            .arg(source)
            .args(["read-tree", "HEAD"])
            .env("GIT_INDEX_FILE", &index)
            .output()
            .map_err(|error| format!("cannot run git to prepare governed view: {error}"))?;
        if !read.status.success() {
            let output = plumb::config::detached("git")
                .arg("-C")
                .arg(source)
                .args(["read-tree", "--empty"])
                .env("GIT_INDEX_FILE", &index)
                .output()
                .map_err(|error| {
                    format!("cannot run git to prepare empty governed view: {error}")
                })?;
            text(output, "prepare empty governed view")?;
        }
        let output = plumb::config::detached("git")
            .arg("-C")
            .arg(source)
            .args(["add", "--all"])
            .env("GIT_INDEX_FILE", &index)
            .output()
            .map_err(|error| format!("cannot run git to capture governed view: {error}"))?;
        text(output, "capture governed view")?;
        let output = plumb::config::detached("git")
            .arg("-C")
            .arg(source)
            .args(["write-tree"])
            .env("GIT_INDEX_FILE", &index)
            .output()
            .map_err(|error| format!("cannot run git to write governed view: {error}"))?;
        let tree = text(output, "write governed view")?;
        Self::new(source, &tree)
    }

    pub(crate) fn govern(&self, profile: &crate::shape::product::Profile) -> Result<(), String> {
        for (name, body) in [
            ("plumb.toml", &profile.manifest),
            ("ectropy.toml", &profile.ectropy),
        ] {
            std::fs::write(self.root.join(name), body)
                .map_err(|error| format!("cannot stage profile {name}: {error}"))?;
        }
        Ok(())
    }
}

fn temporary() -> Result<tempfile::TempDir, String> {
    let base = plumb::config::value("PLUMB_HOME")
        .map(PathBuf::from)
        .or_else(|| plumb::config::data("plumb"))
        .ok_or_else(|| "cannot prepare governed view: no PLUMB_HOME".to_string())?
        .join("tmp");
    std::fs::create_dir_all(&base)
        .map_err(|error| format!("cannot create {}: {error}", base.display()))?;
    tempfile::Builder::new()
        .prefix("view-")
        .tempdir_in(&base)
        .map_err(|error| format!("cannot reserve governed view: {error}"))
}

impl Drop for Index {
    fn drop(&mut self) {
        let _ = plumb::config::detached("git")
            .arg("-C")
            .arg(&self.source)
            .args(["worktree", "remove", "--force"])
            .arg(&self.root)
            .output();
    }
}

pub(super) fn execute(root: &Path, argv: &[String], seat: Option<&Path>) -> Result<(), String> {
    let (program, args) = argv
        .split_first()
        .ok_or_else(|| "guard action has no command".to_string())?;
    let mut command = plumb::config::detached(program);
    command
        .args(args)
        .current_dir(root)
        .env_remove("GIT_INDEX_FILE");
    if let Some(seat) = seat {
        command
            .env_remove("PLUMB_HOME")
            .env("PLUMB_GUARD_CONFIGURATION", seat);
    }
    let status = command
        .status()
        .map_err(|error| format!("cannot run {}: {error}", argv.join(" ")))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{} failed with {status}", argv.join(" ")))
    }
}

pub(super) fn git(root: &Path, args: &[&str], action: &str) -> Result<String, String> {
    let output = plumb::config::detached("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|error| format!("cannot run git to {action}: {error}"))?;
    text(output, action)
}

fn text(output: Output, action: &str) -> Result<String, String> {
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(format!(
            "cannot {action}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}
