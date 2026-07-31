use plumb::boundary::{Request, check};
use std::path::Path;
use std::process::{Command, Output};

struct Repo {
    fixture: tempfile::TempDir,
}

impl Repo {
    fn new() -> Self {
        let fixture = tempfile::tempdir().expect("fixture");
        run(fixture.path(), &["init", "-q"]);
        run(fixture.path(), &["config", "user.name", "Plumb Test"]);
        run(
            fixture.path(),
            &["config", "user.email", "plumb@example.invalid"],
        );
        Self { fixture }
    }

    fn root(&self) -> &Path {
        self.fixture.path()
    }

    fn write(&self, path: &str, text: &str) {
        let path = self.root().join(path);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, text).expect("write");
    }

    fn commit(&self, message: &str) -> String {
        run(self.root(), &["add", "-A"]);
        run(self.root(), &["commit", "-q", "-m", message]);
        text(run(self.root(), &["rev-parse", "HEAD"]))
    }
}

fn run(root: &Path, args: &[&str]) -> Output {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .expect("git");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn text(output: Output) -> String {
    String::from_utf8(output.stdout)
        .expect("utf8")
        .trim()
        .to_owned()
}

fn inspect(repo: &Repo, base: &str, head: &str, write: &[&str]) -> plumb::boundary::Report {
    let write = write
        .iter()
        .map(|path| (*path).to_owned())
        .collect::<Vec<_>>();
    check(Request {
        root: repo.root(),
        base,
        head,
        write: &write,
    })
    .expect("boundary proof")
}

#[test]
fn subset() {
    let repo = Repo::new();
    repo.write("crates/api/a.rs", "one\n");
    repo.write("crates/apricot/a.rs", "one\n");
    let base = repo.commit("base");
    repo.write("crates/api/a.rs", "two\n");
    repo.write("crates/apricot/a.rs", "two\n");
    let head = repo.commit("head");
    let report = inspect(&repo, &base, &head, &["crates/api"]);
    assert_eq!(report.changed, ["crates/api/a.rs", "crates/apricot/a.rs"]);
    assert_eq!(report.outside, ["crates/apricot/a.rs"]);
    assert!(!report.ok);
}

#[test]
fn rename() {
    let repo = Repo::new();
    repo.write("old/item.rs", "same\n");
    let base = repo.commit("base");
    std::fs::create_dir_all(repo.root().join("new")).expect("directory");
    std::fs::rename(
        repo.root().join("old/item.rs"),
        repo.root().join("new/item.rs"),
    )
    .expect("rename");
    let head = repo.commit("rename");
    let report = inspect(&repo, &base, &head, &["new"]);
    assert_eq!(report.changed, ["new/item.rs", "old/item.rs"]);
    assert_eq!(report.outside, ["old/item.rs"]);
}

#[test]
fn copy() {
    let repo = Repo::new();
    repo.write("source/item.rs", "same\n");
    let base = repo.commit("base");
    repo.write("copied/item.rs", "same\n");
    let head = repo.commit("copy");
    let report = inspect(&repo, &base, &head, &["copied"]);
    assert_eq!(report.changed, ["copied/item.rs", "source/item.rs"]);
    assert_eq!(report.outside, ["source/item.rs"]);
}

#[cfg(unix)]
#[test]
fn changes() {
    use std::os::unix::fs::symlink;

    let repo = Repo::new();
    repo.write("deleted", "gone\n");
    repo.write("typed", "file\n");
    let base = repo.commit("base");
    std::fs::remove_file(repo.root().join("deleted")).expect("delete");
    std::fs::remove_file(repo.root().join("typed")).expect("replace");
    symlink("target", repo.root().join("typed")).expect("symlink");
    repo.write("added", "new\n");
    let head = repo.commit("changes");
    let report = inspect(&repo, &base, &head, &["."]);
    assert_eq!(report.changed, ["added", "deleted", "typed"]);
    assert!(report.ok);
}

#[test]
fn gitlink() {
    let child = Repo::new();
    child.write("file", "one\n");
    child.commit("child");
    let repo = Repo::new();
    let source = child.root().to_str().expect("utf8 child");
    run(
        repo.root(),
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            "-q",
            source,
            "vendor/child",
        ],
    );
    let base = repo.commit("base");
    let nested = repo.root().join("vendor/child");
    run(&nested, &["config", "user.name", "Plumb Test"]);
    run(&nested, &["config", "user.email", "plumb@example.invalid"]);
    std::fs::write(nested.join("file"), "two\n").expect("advance child");
    run(&nested, &["add", "file"]);
    run(&nested, &["commit", "-q", "-m", "advance"]);
    let head = repo.commit("advance gitlink");
    let report = inspect(&repo, &base, &head, &["vendor"]);
    assert_eq!(report.changed, ["vendor/child"]);
    assert!(report.ok);
}

#[test]
fn root() {
    let repo = Repo::new();
    repo.write("a", "one\n");
    let base = repo.commit("base");
    repo.write("a", "two\n");
    let head = repo.commit("head");
    let report = inspect(&repo, &base, &head, &[".", "a"]);
    assert_eq!(report.write, ["."]);
    assert!(report.ok);
}

#[test]
fn dirty() {
    let repo = Repo::new();
    repo.write("a", "one\n");
    let base = repo.commit("base");
    repo.write("a", "two\n");
    let head = repo.commit("head");
    repo.write("untracked", "payload\n");
    let write = vec![".".to_owned()];
    let error = check(Request {
        root: repo.root(),
        base: &base,
        head: &head,
        write: &write,
    })
    .expect_err("dirty tree must refuse");
    assert_eq!(error.kind, "dirty");
}

#[test]
fn malformed() {
    let repo = Repo::new();
    repo.write("a", "one\n");
    let head = repo.commit("base");
    let write = vec!["src/../secret".to_owned()];
    let error = check(Request {
        root: repo.root(),
        base: &head,
        head: &head,
        write: &write,
    })
    .expect_err("malformed boundary must refuse");
    assert_eq!(error.kind, "boundary");
}

#[test]
fn oid() {
    let repo = Repo::new();
    repo.write("a", "one\n");
    let head = repo.commit("base");
    let write = vec![".".to_owned()];
    let error = check(Request {
        root: repo.root(),
        base: "HEAD",
        head: &head,
        write: &write,
    })
    .expect_err("symbolic OID must refuse");
    assert_eq!(error.kind, "oid");
}

#[test]
fn history() {
    let repo = Repo::new();
    repo.write("a", "one\n");
    let head = repo.commit("main");
    let branch = text(run(repo.root(), &["branch", "--show-current"]));
    run(repo.root(), &["checkout", "-q", "--orphan", "unrelated"]);
    run(
        repo.root(),
        &["commit", "-q", "--allow-empty", "-m", "unrelated"],
    );
    let base = text(run(repo.root(), &["rev-parse", "HEAD"]));
    run(repo.root(), &["checkout", "-q", &branch]);
    let write = vec![".".to_owned()];
    let error = check(Request {
        root: repo.root(),
        base: &base,
        head: &head,
        write: &write,
    })
    .expect_err("unrelated base must refuse");
    assert_eq!(error.kind, "history");
}

#[cfg(unix)]
#[test]
fn unicode() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let repo = Repo::new();
    repo.write("base", "one\n");
    let base = repo.commit("base");
    let path = OsString::from_vec(vec![b'b', b'a', b'd', b'-', 0xff]);
    std::fs::write(repo.root().join(path), "two\n").expect("raw path");
    let head = repo.commit("head");
    let write = vec![".".to_owned()];
    let error = check(Request {
        root: repo.root(),
        base: &base,
        head: &head,
        write: &write,
    })
    .expect_err("non-UTF-8 path must refuse");
    assert_eq!(error.kind, "git-path");
}
