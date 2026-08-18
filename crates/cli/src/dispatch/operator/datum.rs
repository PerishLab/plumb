use crate::shape::dependency;
use plumb::datum::{self, Datum};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const INDEX: &str = "plumb-datum-index";

pub struct Cut<'a> {
    pub root: &'a Path,
    pub name: &'a str,
    pub version: &'a str,
    pub head: &'a str,
}

pub struct Seat<'a>(pub &'a Path);

pub fn plan(version: &str) -> String {
    format!("record {} on the release line", datum::leaf(version))
}

pub fn record(cut: Cut<'_>) -> Result<String, String> {
    let seat = Seat(cut.root);
    let datum = Datum::new(cut.version, dependency::answers(cut.root)?);
    let count = datum.answers.len();
    seat.reachable(cut.head)?;
    if seat.standing(cut.head, cut.version) {
        return Ok(format!(
            "{} already stands on the release line",
            datum::leaf(cut.version)
        ));
    }
    let object = seat.blob(&datum.encode()?)?;
    let tree = seat.staged(cut.head, &object, cut.version)?;
    let commit = seat.sealed(&tree, cut.head, cut.version)?;
    seat.push(&commit, cut.name)?;
    Ok(format!(
        "recorded {} with {count} answers",
        datum::leaf(cut.version)
    ))
}

impl Seat<'_> {
    pub fn carried(&self, commit: &str, version: &str) -> bool {
        let seat = format!("{}/{version}/", datum::SEAT);
        let Ok(touched) = read(
            "inspect datum commit",
            self.command(["show", "--name-only", "--format=", commit]),
        ) else {
            return false;
        };
        let owned = touched
            .lines()
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .all(|path| path.starts_with(&seat));
        owned && self.decodes(&format!("{commit}:{}", datum::leaf(version)), version)
    }

    fn reachable(&self, head: &str) -> Result<(), String> {
        let seen = self
            .command(["cat-file", "-e", &format!("{head}^{{commit}}")])
            .is_ok_and(|output| output.status.success());
        if seen {
            Ok(())
        } else {
            Err(format!(
                "the release line stands at {head}, which this checkout does not hold"
            ))
        }
    }

    fn standing(&self, head: &str, version: &str) -> bool {
        self.decodes(&format!("{head}:{}", datum::leaf(version)), version)
    }

    fn decodes(&self, object: &str, version: &str) -> bool {
        self.command(["show", object]).is_ok_and(|output| {
            output.status.success() && datum::decode(version, &output.stdout).is_ok()
        })
    }

    fn blob(&self, text: &str) -> Result<String, String> {
        let mut child = Command::new("git")
            .arg("-C")
            .arg(self.0)
            .args(["hash-object", "-w", "--stdin"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .map_err(|error| format!("cannot run git: {error}"))?;
        let mut sink = child
            .stdin
            .take()
            .ok_or_else(|| "cannot write the datum to git".to_string())?;
        std::io::Write::write_all(&mut sink, text.as_bytes())
            .map_err(|error| format!("cannot write the datum to git: {error}"))?;
        drop(sink);
        let output = child
            .wait_with_output()
            .map_err(|error| format!("cannot run git: {error}"))?;
        read("write the datum object", Ok(output))
    }

    fn staged(&self, head: &str, object: &str, version: &str) -> Result<String, String> {
        let index = self.index()?;
        let _ = std::fs::remove_file(&index);
        let stage = |args: Vec<String>| -> Result<Output, String> {
            Command::new("git")
                .arg("-C")
                .arg(self.0)
                .args(args)
                .env("GIT_INDEX_FILE", &index)
                .output()
                .map_err(|error| format!("cannot run git: {error}"))
        };
        success("read the release tree", stage(owned(["read-tree", head])))?;
        for stale in self.stale(head, version)? {
            success(
                "drop a stale datum",
                stage(owned(["update-index", "--force-remove", &stale])),
            )?;
        }
        success(
            "stage the datum",
            stage(owned([
                "update-index",
                "--add",
                "--cacheinfo",
                &format!("100644,{object},{}", datum::leaf(version)),
            ])),
        )?;
        let tree = read("write the datum tree", stage(owned(["write-tree"])));
        let _ = std::fs::remove_file(&index);
        tree
    }

    fn stale(&self, head: &str, version: &str) -> Result<Vec<String>, String> {
        let seat = format!("{}/{version}", datum::SEAT);
        let leaf = datum::leaf(version);
        let listed = read(
            "list the datum seat",
            self.command(["ls-tree", "-r", "--name-only", head, "--", &seat]),
        )?;
        Ok(listed
            .lines()
            .map(str::trim)
            .filter(|path| !path.is_empty() && *path != leaf)
            .map(str::to_string)
            .collect())
    }

    fn sealed(&self, tree: &str, head: &str, version: &str) -> Result<String, String> {
        let message = format!("Record the datum {version} judges against");
        read(
            "commit the datum",
            self.command(["commit-tree", tree, "-p", head, "-m", &message]),
        )
    }

    fn push(&self, commit: &str, name: &str) -> Result<(), String> {
        success(
            "push the datum",
            self.command(["push", "origin", &format!("{commit}:refs/heads/{name}")]),
        )
    }

    fn index(&self) -> Result<PathBuf, String> {
        let dir = read(
            "resolve git directory",
            self.command(["rev-parse", "--git-dir"]),
        )?;
        let dir = PathBuf::from(&dir);
        Ok(if dir.is_absolute() {
            dir.join(INDEX)
        } else {
            self.0.join(dir).join(INDEX)
        })
    }

    fn command<const N: usize>(&self, args: [&str; N]) -> Result<Output, String> {
        Command::new("git")
            .arg("-C")
            .arg(self.0)
            .args(args)
            .output()
            .map_err(|error| format!("cannot run git: {error}"))
    }
}

fn owned<const N: usize>(args: [&str; N]) -> Vec<String> {
    args.iter().map(|arg| (*arg).to_string()).collect()
}

fn success(action: &str, output: Result<Output, String>) -> Result<(), String> {
    read(action, output).map(|_| ())
}

fn read(action: &str, output: Result<Output, String>) -> Result<String, String> {
    let output = output?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(format!(
            "{action} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}
