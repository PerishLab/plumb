use crate::command::ship::adaptor;
use crate::shape::release::Spec;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub fn plan(version: &str, line: &str) -> String {
    format!("project repository version {version} on {line}")
}

pub fn project(root: &Path, line: &str, version: &str, head: &str) -> Result<String, String> {
    let tree = Tree::open(root, head)?;
    let spec = Spec::read(&tree.seat.join("plumb.toml"))?;
    adaptor::registry::registry(&spec).prepare(version)?;
    adaptor::module::module(&spec).prepare(version)?;
    adaptor::chart::chart(&spec).prepare(version)?;
    if tree.clean()? {
        return Ok(head.to_string());
    }
    tree.commit(line, version)
}

struct Tree {
    root: PathBuf,
    seat: PathBuf,
    temp: tempfile::TempDir,
}

impl Tree {
    fn open(root: &Path, head: &str) -> Result<Self, String> {
        let temp = tempfile::tempdir()
            .map_err(|error| format!("cannot open release version seat: {error}"))?;
        let seat = temp.path().join("source");
        success(
            "open release version worktree",
            Command::new("git")
                .arg("-C")
                .arg(root)
                .args(["worktree", "add", "--detach"])
                .arg(&seat)
                .arg(head)
                .output(),
        )?;
        Ok(Self {
            root: root.to_path_buf(),
            seat,
            temp,
        })
    }

    fn clean(&self) -> Result<bool, String> {
        read(
            "inspect release version worktree",
            self.git(["status", "--porcelain"]),
        )
        .map(|held| held.is_empty())
    }

    fn commit(&self, line: &str, version: &str) -> Result<String, String> {
        success("stage release version", self.git(["add", "-A"]))?;
        success(
            "commit release version",
            self.git(["commit", "-m", &format!("Prepare {version}")]),
        )?;
        let commit = read(
            "resolve release version commit",
            self.git(["rev-parse", "HEAD"]),
        )?;
        success(
            "push release version",
            self.git(["push", "origin", &format!("{commit}:refs/heads/{line}")]),
        )?;
        Ok(commit)
    }

    fn git<const N: usize>(&self, args: [&str; N]) -> Result<Output, std::io::Error> {
        Command::new("git")
            .arg("-C")
            .arg(&self.seat)
            .args(args)
            .output()
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args(["worktree", "remove", "--force"])
            .arg(&self.seat)
            .output();
        let _ = &self.temp;
    }
}

fn success(action: &str, output: Result<Output, std::io::Error>) -> Result<(), String> {
    read(action, output).map(|_| ())
}

fn read(action: &str, output: Result<Output, std::io::Error>) -> Result<String, String> {
    let output = output.map_err(|error| format!("cannot {action}: {error}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(format!(
            "{action} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}
