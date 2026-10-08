use super::state::State;
use std::path::Path;

pub(super) struct Work<'a> {
    pub source: &'a Path,
    pub state: &'a State,
}

impl Work<'_> {
    pub fn open(&self) -> Result<(), String> {
        if let Ok(metadata) = std::fs::symlink_metadata(&self.state.worktree)
            && metadata.file_type().is_symlink()
        {
            return Err("Auto worktree seat is a symlink".into());
        }
        let branch = format!("auto/{}", self.state.issue);
        let created = !self.state.worktree.exists();
        if created {
            let start = self.state.pushed.as_deref().unwrap_or("origin/main");
            let exists = std::process::Command::new("git")
                .arg("-C")
                .arg(self.source)
                .args([
                    "show-ref",
                    "--verify",
                    "--quiet",
                    &format!("refs/heads/{branch}"),
                ])
                .status()
                .map_err(|error| error.to_string())?
                .success();
            let path = self
                .state
                .worktree
                .to_str()
                .ok_or("Auto worktree path is not UTF-8")?;
            let args = if exists {
                if git(self.source, &["rev-parse", &branch])?
                    != git(self.source, &["rev-parse", start])?
                {
                    return Err("existing Auto branch is outside recorded recovery state".into());
                }
                vec!["worktree", "add", path, &branch]
            } else {
                vec!["worktree", "add", "-b", &branch, path, start]
            };
            git(self.source, &args)?;
        }
        let actual = git(&self.state.worktree, &["branch", "--show-current"])?;
        if actual != branch {
            return Err("Auto worktree branch disagrees with declared state".into());
        }
        let common = git(
            &self.state.worktree,
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
        )?;
        if common
            != git(
                self.source,
                &["rev-parse", "--path-format=absolute", "--git-common-dir"],
            )?
        {
            return Err("Auto worktree repository disagrees".into());
        }
        let dir = git(&self.state.worktree, &["rev-parse", "--absolute-git-dir"])?;
        let marker = Path::new(&dir).join("plumb-auto.json");
        let identity = serde_json::to_vec(&(
            "plumb.auto-worktree/v1",
            &self.state.repository,
            self.state.issue,
        ))
        .map_err(|error| error.to_string())?;
        if created {
            return std::fs::write(marker, identity).map_err(|error| error.to_string());
        }
        let held = std::fs::read(marker).map_err(|error| {
            format!("existing Auto worktree has no readable ownership marker: {error}")
        })?;
        if held != identity {
            return Err("Auto worktree marker disagrees".into());
        }
        Ok(())
    }

    pub fn commit(&self) -> Result<String, String> {
        let root = &self.state.worktree;
        let tree = plumb::guard::tree(root)?;
        let head = git(root, &["rev-parse", "HEAD"])?;
        if tree == git(root, &["rev-parse", "HEAD^{tree}"])? {
            return Ok(head);
        }
        let message = format!(
            "Follow the latest first-party packages\n\nRefs #{}.\n",
            self.state.issue
        );
        let output = std::process::Command::new("git")
            .args([
                "-C",
                root.to_str().ok_or("worktree path is not UTF-8")?,
                "commit-tree",
                &tree,
                "-p",
                &head,
                "-m",
                &message,
            ])
            .env("GIT_AUTHOR_NAME", "Plumb Auto")
            .env("GIT_AUTHOR_EMAIL", "auto@plumb.invalid")
            .env("GIT_COMMITTER_NAME", "Plumb Auto")
            .env("GIT_COMMITTER_EMAIL", "auto@plumb.invalid")
            .output()
            .map_err(|error| error.to_string())?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).into_owned());
        }
        let commit = String::from_utf8(output.stdout)
            .map_err(|error| error.to_string())?
            .trim()
            .to_string();
        let branch = format!("refs/heads/auto/{}", self.state.issue);
        git(root, &["update-ref", &branch, &commit, &head])?;
        Ok(commit)
    }
}

pub(super) fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let mut command = plumb::config::detached("git");
    command
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0");
    let bytes = super::provider::capture(command, std::time::Duration::from_secs(120))?;
    String::from_utf8(bytes)
        .map(|text| text.trim().into())
        .map_err(|error| format!("Auto git output is not UTF-8: {error}"))
}
