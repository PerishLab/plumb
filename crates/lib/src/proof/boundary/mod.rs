use serde::Serialize;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

mod path;

pub const SCHEMA: &str = "plumb.precommit/v1";

pub struct Request<'a> {
    pub root: &'a Path,
    pub base: &'a str,
    pub head: &'a str,
    pub write: &'a [String],
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Report {
    pub schema: &'static str,
    pub root: PathBuf,
    pub base: String,
    pub head: String,
    pub write: Vec<String>,
    pub changed: Vec<String>,
    pub outside: Vec<String>,
    pub clean: bool,
    pub ok: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Refusal {
    pub kind: &'static str,
    pub message: String,
}

struct Repository {
    root: PathBuf,
}

struct Claim(Vec<String>);

struct Delta;

impl Display for Refusal {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for Refusal {}

pub fn check(request: Request<'_>) -> Result<Report, Refusal> {
    let repo = Repository::open(request.root)?;
    let claim = Claim::new(request.write)?;
    let base = repo.commit(request.base, "base")?;
    let head = repo.commit(request.head, "head")?;
    repo.head(&head)?;
    repo.clean()?;
    repo.ancestor(&base, &head)?;
    let changed = repo.delta(&base, &head)?;
    let outside = changed
        .iter()
        .filter(|path| !claim.holds(path))
        .cloned()
        .collect::<Vec<_>>();
    Ok(Report {
        schema: SCHEMA,
        root: repo.root,
        base,
        head,
        write: claim.0,
        changed,
        clean: true,
        ok: outside.is_empty(),
        outside,
    })
}

impl Repository {
    fn open(path: &Path) -> Result<Self, Refusal> {
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

    fn commit(&self, oid: &str, name: &str) -> Result<String, Refusal> {
        if !exact(oid) {
            return Err(refuse(
                "oid",
                format!("{name} must be an exact lowercase commit OID"),
            ));
        }
        let spec = format!("{oid}^{{commit}}");
        let output = self.git(&["rev-parse", "--verify", &spec])?;
        success(output, "oid", format!("cannot resolve {name} commit"))
            .and_then(|bytes| line(bytes, "oid"))
    }

    fn head(&self, expected: &str) -> Result<(), Refusal> {
        let output = self.git(&["rev-parse", "--verify", "HEAD^{commit}"])?;
        let current = line(
            success(output, "head", "cannot resolve repository HEAD")?,
            "head",
        )?;
        if current != expected {
            return Err(refuse(
                "head",
                format!("repository HEAD is {current}, not {expected}"),
            ));
        }
        Ok(())
    }

    fn clean(&self) -> Result<(), Refusal> {
        let output = self.git(&["status", "--porcelain=v1", "-z", "--untracked-files=all"])?;
        let bytes = success(output, "dirty", "cannot inspect working tree")?;
        if !bytes.is_empty() {
            return Err(refuse(
                "dirty",
                "working tree contains tracked or untracked changes",
            ));
        }
        Ok(())
    }

    fn ancestor(&self, base: &str, head: &str) -> Result<(), Refusal> {
        let output = self.git(&["merge-base", "--is-ancestor", base, head])?;
        if output.status.success() {
            return Ok(());
        }
        if output.status.code() == Some(1) {
            return Err(refuse("history", "base is not an ancestor of head"));
        }
        Err(refuse(
            "git",
            stderr(output, "cannot inspect commit ancestry"),
        ))
    }

    fn delta(&self, base: &str, head: &str) -> Result<Vec<String>, Refusal> {
        let output = self.git(&[
            "diff",
            "--name-status",
            "-z",
            "--find-renames",
            "--find-copies",
            "--find-copies-harder",
            base,
            head,
            "--",
        ])?;
        let bytes = success(output, "git", "cannot read committed tree delta")?;
        Delta::parse(&bytes)
    }

    fn git(&self, args: &[&str]) -> Result<Output, Refusal> {
        Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args(args)
            .output()
            .map_err(|error| refuse("git", format!("cannot execute git: {error}")))
    }
}

impl Claim {
    fn new(write: &[String]) -> Result<Self, Refusal> {
        if write.is_empty() {
            return Err(refuse(
                "boundary",
                "at least one write boundary is required",
            ));
        }
        let mut held = write
            .iter()
            .map(|path| path::parse(path, true))
            .collect::<Result<Vec<_>, _>>()?;
        held.sort();
        held.dedup();
        let mut reduced: Vec<String> = Vec::new();
        for path in held {
            if !reduced.iter().any(|parent| covers(parent, &path)) {
                reduced.push(path);
            }
        }
        Ok(Self(reduced))
    }

    fn holds(&self, path: &str) -> bool {
        self.0.iter().any(|boundary| covers(boundary, path))
    }
}

impl Delta {
    fn parse(bytes: &[u8]) -> Result<Vec<String>, Refusal> {
        let mut fields = bytes
            .split(|byte| *byte == 0)
            .filter(|field| !field.is_empty());
        let mut paths = Vec::new();
        while let Some(status) = fields.next() {
            let status = std::str::from_utf8(status)
                .map_err(|_| refuse("git-path", "Git emitted a non-UTF-8 status"))?;
            let count = usize::from(status.starts_with('R') || status.starts_with('C')) + 1;
            for _ in 0..count {
                let field = fields.next().ok_or_else(|| {
                    refuse("git", format!("Git emitted an incomplete {status} record"))
                })?;
                let path = std::str::from_utf8(field)
                    .map_err(|_| refuse("git-path", "Git delta contains a non-UTF-8 path"))?;
                paths.push(path::parse(path, false)?);
            }
        }
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}

fn covers(boundary: &str, path: &str) -> bool {
    boundary == "."
        || boundary == path
        || path
            .strip_prefix(boundary)
            .is_some_and(|tail| tail.starts_with('/'))
}

fn exact(oid: &str) -> bool {
    matches!(oid.len(), 40 | 64)
        && oid
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn success(
    output: Output,
    kind: &'static str,
    fallback: impl Into<String>,
) -> Result<Vec<u8>, Refusal> {
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(refuse(kind, stderr(output, &fallback.into())))
    }
}

fn line(bytes: Vec<u8>, kind: &'static str) -> Result<String, Refusal> {
    String::from_utf8(bytes)
        .map(|line| line.trim().to_owned())
        .map_err(|_| refuse(kind, "Git emitted non-UTF-8 evidence"))
}

fn stderr(output: Output, fallback: &str) -> String {
    let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    if message.is_empty() {
        fallback.to_owned()
    } else {
        message
    }
}

fn refuse(kind: &'static str, message: impl Into<String>) -> Refusal {
    Refusal {
        kind,
        message: message.into(),
    }
}
