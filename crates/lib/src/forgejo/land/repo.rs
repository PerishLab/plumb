use super::{Refusal, refuse};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

pub struct Repository {
    pub root: PathBuf,
}

impl Repository {
    pub fn open(path: &Path) -> Result<Self, Refusal> {
        let root = std::fs::canonicalize(path).map_err(|error| {
            refuse(
                "root",
                format!("cannot resolve {}: {error}", path.display()),
            )
        })?;
        if !root.is_dir() {
            return Err(refuse(
                "root",
                format!("{} is not a directory", root.display()),
            ));
        }
        Ok(Self { root })
    }

    pub fn git(&self, args: &[&str]) -> Result<Output, Refusal> {
        Command::new("git")
            .args(args)
            .current_dir(&self.root)
            .output()
            .map_err(|error| refuse("git", format!("cannot run git: {error}")))
    }

    pub fn record(&self, seed: &Seed<'_>) -> Result<String, Refusal> {
        use std::io::Write as _;
        let mut child = Command::new("git")
            .args(["commit-tree", seed.tree, "-p", seed.parent, "-F", "-"])
            .current_dir(&self.root)
            .envs(seed.identity)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| refuse("git", format!("cannot run git commit-tree: {error}")))?;
        child
            .stdin
            .take()
            .ok_or_else(|| refuse("git", "cannot open git commit-tree input"))?
            .write_all(seed.message.as_bytes())
            .map_err(|error| refuse("git", format!("cannot write commit message: {error}")))?;
        let output = child
            .wait_with_output()
            .map_err(|error| refuse("git", format!("cannot wait for git commit-tree: {error}")))?;
        line(success(
            output,
            "candidate",
            "cannot write the candidate commit",
        )?)
    }

    pub fn text(&self, args: &[&str], kind: &'static str, whose: &str) -> Result<String, Refusal> {
        let output = self.git(args)?;
        let bytes = success(output, kind, whose)?;
        Ok(String::from_utf8_lossy(&bytes).trim_end().to_string())
    }

    pub fn revision(&self, reference: &str) -> Result<String, Refusal> {
        let output = self.git(&["rev-parse", "--verify", reference])?;
        let bytes = success(output, "revision", format!("cannot resolve {reference}"))?;
        line(bytes)
    }

    pub fn verified(&self, reference: &str) -> bool {
        self.git(&["rev-parse", "--verify", reference])
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    pub fn branch(&self) -> Result<String, Refusal> {
        let name = self.text(
            &["branch", "--show-current"],
            "detached",
            "cannot read the current branch",
        )?;
        if name.is_empty() {
            return Err(refuse(
                "detached",
                "detached HEAD is not a landable topic branch",
            ));
        }
        Ok(name)
    }

    pub fn clean(&self) -> Result<(), Refusal> {
        let output = self.git(&["status", "--porcelain=v1", "-z", "--untracked-files=all"])?;
        let bytes = success(output, "dirty", "cannot inspect working tree")?;
        if !bytes.is_empty() {
            return Err(refuse(
                "dirty",
                "working tree must be clean; commit or discard changes first",
            ));
        }
        Ok(())
    }

    pub fn fetch(&self) -> Result<(), Refusal> {
        let output = self.git(&["fetch", "--prune", "origin"])?;
        success(output, "git", "cannot fetch origin").map(|_| ())
    }

    pub fn seat(&self, branch: &str) -> Result<Option<PathBuf>, Refusal> {
        let listed = self.text(
            &["worktree", "list", "--porcelain"],
            "git",
            "cannot list worktrees",
        )?;
        let wanted = format!("branch refs/heads/{branch}");
        let mut seen: Option<PathBuf> = None;
        for raw in listed.lines() {
            let held = raw.trim();
            if let Some(path) = held.strip_prefix("worktree ") {
                seen = Some(PathBuf::from(path.trim()));
            }
            if held == wanted
                && let Some(path) = seen.clone()
                && !self.owns(&path)
            {
                return Ok(Some(path));
            }
        }
        Ok(None)
    }

    fn owns(&self, path: &Path) -> bool {
        std::fs::canonicalize(path)
            .map(|held| held == self.root)
            .unwrap_or(false)
    }
}

pub struct Seed<'a> {
    pub tree: &'a str,
    pub parent: &'a str,
    pub message: &'a str,
    pub identity: &'a BTreeMap<String, String>,
}

pub fn success(
    output: Output,
    kind: &'static str,
    whose: impl Into<String>,
) -> Result<Vec<u8>, Refusal> {
    if output.status.success() {
        return Ok(output.stdout);
    }
    let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let whose = whose.into();
    Err(refuse(
        kind,
        if detail.is_empty() {
            whose
        } else {
            format!("{whose}: {detail}")
        },
    ))
}

pub fn line(bytes: Vec<u8>) -> Result<String, Refusal> {
    let text = String::from_utf8_lossy(&bytes).trim().to_string();
    if text.is_empty() {
        return Err(refuse("git", "git returned no output"));
    }
    Ok(text)
}
