use crate::command::ship::adaptor;
use crate::shape::release::Spec;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub fn plan(version: &str, line: &str) -> String {
    format!("project repository version {version} on {line}")
}

pub fn project(root: &Path, line: &str, version: &str, head: &str) -> Result<String, String> {
    let tree = Tree::open(root, head)?;
    tree.prepare(version)?;
    if tree.clean()? {
        return Ok(head.to_string());
    }
    tree.commit(line, version)
}

pub(super) fn prove(root: &Path, line: &str, version: &str, head: &str) -> Result<String, String> {
    if plumb::guard::current(root, head).is_ok() {
        return Ok(head.to_string());
    }
    let body = read(
        "read release proof",
        Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["show", "-s", "--format=%B", head])
            .output(),
    )?;
    if !body
        .lines()
        .any(|line| line.starts_with(plumb::guard::TRAILER))
    {
        return Ok(head.to_string());
    }
    Tree::open(root, head)?.prove(line, version, head)
}

pub struct Preparation<'a> {
    pub root: &'a Path,
    pub commit: &'a str,
    pub base: &'a str,
    pub version: &'a str,
    pub body: &'a str,
}

pub fn prepared(cut: Preparation<'_>) -> bool {
    if message(cut.body) != format!("Prepare {}", cut.version) {
        return false;
    }
    let parent = read(
        "resolve release preparation parent",
        Command::new("git")
            .arg("-C")
            .arg(cut.root)
            .args(["rev-parse", &format!("{}^", cut.commit)])
            .output(),
    );
    if parent.as_deref() != Ok(cut.base) {
        return false;
    }
    let Ok(tree) = Tree::open(cut.root, cut.base) else {
        return false;
    };
    if tree.prepare(cut.version).is_err() {
        return false;
    }
    let expected = tree.tree();
    let actual = read(
        "resolve release preparation tree",
        Command::new("git")
            .arg("-C")
            .arg(cut.root)
            .args(["rev-parse", &format!("{}^{{tree}}", cut.commit)])
            .output(),
    );
    let original = read(
        "resolve release base tree",
        Command::new("git")
            .arg("-C")
            .arg(cut.root)
            .args(["rev-parse", &format!("{}^{{tree}}", cut.base)])
            .output(),
    );
    expected.is_ok() && expected != original && expected == actual
}

fn message(body: &str) -> String {
    body.lines()
        .filter(|line| !line.starts_with(plumb::guard::TRAILER))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
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

    fn prepare(&self, version: &str) -> Result<(), String> {
        let spec = Spec::resolve(&self.seat)?;
        adaptor::registry::registry(&spec).prepare(version)?;
        adaptor::module::module(&spec).prepare(version)?;
        adaptor::chart::chart(&spec).prepare(version)
    }

    fn tree(&self) -> Result<String, String> {
        success("stage release version", self.git(["add", "-A"]))?;
        read("write release version tree", self.git(["write-tree"]))
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

    fn prove(&self, line: &str, version: &str, head: &str) -> Result<String, String> {
        let proof = crate::command::guard::precommit::proof(&self.seat)?;
        let tree = read(
            "resolve release proof tree",
            self.git(["rev-parse", "HEAD^{tree}"]),
        )?;
        if proof.tree != tree {
            return Err(format!(
                "release proof seals {}, not the standing tree {tree}",
                proof.tree
            ));
        }
        let message = format!(
            "Refresh the release proof for {version}\n\n{} {}",
            plumb::guard::TRAILER,
            proof.encode()?
        );
        let commit = read(
            "commit refreshed release proof",
            self.git(["commit-tree", &tree, "-p", head, "-m", &message]),
        )?;
        success(
            "push refreshed release proof",
            self.git(["push", "origin", &format!("{commit}:refs/heads/{line}")]),
        )?;
        Ok(commit)
    }

    fn git<const N: usize>(&self, args: [&str; N]) -> Result<Output, std::io::Error> {
        plumb::config::current("git")
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
