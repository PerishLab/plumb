use crate::command::ship::adaptor;
use crate::shape::release::Spec;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub fn plan(version: &str, line: &str) -> String {
    format!("project repository version {version} on {line}")
}

pub fn project(root: &Path, version: &str, head: &str) -> Result<String, String> {
    let tree = Tree::open(root, head)?;
    tree.prepare(version)?;
    if tree.clean()? {
        return Ok(head.to_string());
    }
    tree.commit(version, head)
}

pub fn publish(root: &Path, line: &str, head: &str) -> Result<(), String> {
    success(
        "push the proved version projection",
        plumb::config::current("git")
            .arg("-C")
            .arg(root)
            .args(["push", "origin", &format!("{head}:refs/heads/{line}")])
            .output(),
    )
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
        .filter(|line| {
            !line.starts_with(plumb::guard::TRAILER) && !line.starts_with(plumb::datum::TRAILER)
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

pub(super) struct Tree {
    root: PathBuf,
    seat: PathBuf,
    temp: tempfile::TempDir,
}

impl Tree {
    pub(super) fn open(root: &Path, head: &str) -> Result<Self, String> {
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
        adaptor::chart::chart(&spec).prepare(version)?;
        self.retire(version)
    }

    pub(super) fn resolve(&self, version: &str, head: &str) -> Result<plumb::datum::Datum, String> {
        super::datum::resolve(&self.seat, version, head)
    }

    fn retire(&self, version: &str) -> Result<(), String> {
        if !self.migrated(version)? {
            let seat = format!("{}/{version}", plumb::datum::SEAT);
            success("retire legacy datum", self.git(["rm", "-r", "--", &seat]))?;
        }
        Ok(())
    }

    pub(super) fn migrated(&self, version: &str) -> Result<bool, String> {
        let seat = format!("{}/{version}", plumb::datum::SEAT);
        read("inspect legacy datum", self.git(["ls-files", "--", &seat]))
            .map(|paths| paths.is_empty())
    }

    pub(super) fn proved(&self, head: &str) -> Result<bool, String> {
        let body = read(
            "read datum proof",
            self.git(["show", "-s", "--format=%B", head]),
        )?;
        Ok(!body
            .lines()
            .any(|line| line.starts_with(plumb::guard::TRAILER))
            || plumb::guard::current(&self.seat, head).is_ok())
    }

    pub(super) fn datum(&self, head: &str, datum: &plumb::datum::Datum) -> Result<String, String> {
        self.retire(&datum.version)?;
        let message = format!(
            "Record the datum {} judges against\n\n{}",
            datum.version,
            datum.trailer()?
        );
        self.record(head, message)
    }

    fn record(&self, head: &str, mut message: String) -> Result<String, String> {
        let tree = self.tree()?;
        let parent = read(
            "read parent proof",
            self.git(["show", "-s", "--format=%B", head]),
        )?;
        let captured = read(
            "capture the datum for Guard",
            self.git(["commit-tree", &tree, "-p", head, "-m", &message]),
        )?;
        success(
            "select captured datum",
            self.git(["checkout", "--detach", &captured]),
        )?;
        if parent
            .lines()
            .any(|line| line.starts_with(plumb::guard::TRAILER))
        {
            let proof = crate::command::precommit::proof(&self.seat)?;
            if proof.tree != tree {
                return Err("datum proof differs from the staged tree".into());
            }
            message.push_str(&format!("\n{} {}", plumb::guard::TRAILER, proof.encode()?));
        }
        let commit = read(
            "commit the datum",
            self.git(["commit-tree", &tree, "-p", head, "-m", &message]),
        )?;
        Ok(commit)
    }

    fn tree(&self) -> Result<String, String> {
        success("stage release version", self.git(["add", "-A"]))?;
        read("write release version tree", self.git(["write-tree"]))
    }

    fn commit(&self, version: &str, head: &str) -> Result<String, String> {
        let datum = self.resolve(version, head)?;
        self.record(head, format!("Prepare {version}\n\n{}", datum.trailer()?))
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
