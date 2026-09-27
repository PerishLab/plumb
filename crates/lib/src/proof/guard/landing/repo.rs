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
        let mut args = vec!["commit-tree", seed.tree];
        for parent in seed.parents {
            args.extend(["-p", parent]);
        }
        args.extend(["-F", "-"]);
        let mut child = Command::new("git")
            .args(&args)
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
        let bytes = success(self.git(args)?, kind, whose)?;
        Ok(String::from_utf8_lossy(&bytes).trim_end().to_string())
    }

    pub fn revision(&self, reference: &str) -> Result<String, Refusal> {
        let output = self.git(&["rev-parse", "--verify", reference])?;
        line(success(
            output,
            "revision",
            format!("cannot resolve {reference}"),
        )?)
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
}

pub struct Seed<'a> {
    pub tree: &'a str,
    pub parents: &'a [&'a str],
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
