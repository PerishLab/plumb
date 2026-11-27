use plumb::snapshot::Snapshot;
use plumb::vocabulary::{Dictionary, decode, encode, scan};
use std::path::Path;
use std::process::Command;

fn dictionary(encoded: &[String]) -> Dictionary {
    let values = encoded
        .iter()
        .map(|value| format!("\"{value}\""))
        .collect::<Vec<_>>()
        .join(", ");
    Dictionary::parse(&format!("retired = [{values}]\n")).expect("dictionary")
}

fn git(root: &Path, args: &[&str]) -> std::process::Output {
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

fn repo() -> tempfile::TempDir {
    let fixture = tempfile::tempdir().expect("fixture");
    git(fixture.path(), &["init", "-q"]);
    fixture
}

#[test]
fn codec() {
    let encoded = encode("retired-term").expect("encode");
    assert!(encoded.starts_with('~'));
    assert!(!encoded.contains('='));
    assert_eq!(decode(&encoded).expect("decode"), "retired-term");
    assert!(decode("~UmV0aXJlZA").is_err());
    assert!(decode("UmV0aXJlZA").is_err());
}

#[test]
fn duplicate() {
    let encoded = encode("retired").expect("encode");
    let source = format!("retired = [\"{encoded}\", \"{encoded}\"]\n");
    assert_eq!(
        Dictionary::parse(&source).expect_err("duplicate").kind,
        "dictionary"
    );
}

#[test]
fn empty() {
    let fixture = tempfile::tempdir().expect("fixture");
    let report = scan(fixture.path(), &dictionary(&[])).expect("empty report");
    assert!(report.ok);
    assert_eq!(report.retired, 0);
    assert_eq!(report.coverage.tracked, 0);
}

#[test]
fn digest() {
    let fixture = tempfile::tempdir().expect("fixture");
    let first = Dictionary::parse("retired = []\n").expect("first dictionary");
    let second = Dictionary::parse("retired = []\n\n").expect("second dictionary");
    let first = scan(fixture.path(), &first).expect("first report");
    let second = scan(fixture.path(), &second).expect("second report");

    assert_eq!(first.digest.len(), 64);
    assert_ne!(first.digest, second.digest);
}

#[test]
fn closure() {
    let fixture = repo();
    let root = fixture.path();
    std::fs::create_dir_all(root.join("docs/CHANGELOG/v1")).expect("changelog");
    std::fs::create_dir(root.join("src")).expect("src");
    std::fs::write(root.join("src/RETIRED-name.rs"), "a retired value\n").expect("source");
    std::fs::write(root.join("docs/CHANGELOG/v1/INDEX.md"), "retired\n").expect("history");
    git(root, &["add", "-A"]);
    let report = scan(root, &dictionary(&[encode("retired").expect("encode")])).expect("scan");
    assert!(!report.ok);
    assert_eq!(report.coverage.tracked, 2);
    assert_eq!(report.coverage.scanned, 2);
    assert_eq!(report.coverage.exempt, 0);
    assert_eq!(report.hits.len(), 3);
    assert!(report.hits.iter().any(|hit| hit.surface == "path"));
    assert!(report.hits.iter().any(|hit| hit.surface == "content"));
}

#[cfg(unix)]
#[test]
fn symlink() {
    use std::os::unix::fs::symlink;

    let fixture = repo();
    let root = fixture.path();
    std::fs::write(root.join("outside"), "clean\n").expect("target");
    symlink("retired-target", root.join("link")).expect("symlink");
    git(root, &["add", "link"]);
    let report = scan(root, &dictionary(&[encode("retired").expect("encode")])).expect("scan");
    assert_eq!(report.hits.len(), 1);
    assert_eq!(report.hits[0].surface, "content");
}

#[test]
fn deleted() {
    let fixture = repo();
    let root = fixture.path();
    std::fs::write(root.join("file"), "clean\n").expect("file");
    git(root, &["add", "file"]);
    std::fs::remove_file(root.join("file")).expect("delete");
    let error = scan(root, &dictionary(&[encode("retired").expect("encode")]))
        .expect_err("missing tracked file");
    assert_eq!(error.kind, "tracked");
}

#[cfg(unix)]
#[test]
fn unicode() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let fixture = repo();
    let root = fixture.path();
    let path = OsString::from_vec(vec![b'b', b'a', b'd', b'-', 0xff]);
    std::fs::write(root.join(path), "clean\n").expect("raw path");
    git(root, &["add", "-A"]);
    let error = scan(root, &dictionary(&[encode("retired").expect("encode")]))
        .expect_err("non-UTF-8 tracked path");
    assert_eq!(error.kind, "git-path");
}

#[test]
fn gitlink() {
    let child = tempfile::tempdir().expect("child");
    git(child.path(), &["init", "-q"]);
    git(child.path(), &["config", "user.name", "Plumb Test"]);
    git(
        child.path(),
        &["config", "user.email", "plumb@example.invalid"],
    );
    std::fs::write(child.path().join("retired"), "retired\n").expect("child file");
    git(child.path(), &["add", "retired"]);
    git(child.path(), &["commit", "-q", "-m", "child"]);
    let oid = String::from_utf8(git(child.path(), &["rev-parse", "HEAD"]).stdout)
        .expect("oid")
        .trim()
        .to_owned();

    let fixture = repo();
    let root = fixture.path();
    let cache = format!("160000,{oid},vendor/child");
    git(root, &["update-index", "--add", "--cacheinfo", &cache]);
    let report = scan(root, &dictionary(&[encode("retired").expect("encode")])).expect("scan");
    assert!(report.ok);
    assert_eq!(report.coverage.scanned, 1);
    assert!(report.hits.is_empty());
}

#[test]
fn missing() {
    let fixture = repo();
    let root = fixture.path();
    std::fs::write(root.join("ectropy.toml"), "policy = 1\n").expect("ectropy");
    git(root, &["add", "ectropy.toml"]);
    std::fs::remove_file(root.join("ectropy.toml")).expect("delete");

    let error = Snapshot::read(root).expect_err("unstaged deletion must refuse");
    assert_eq!(error.kind, "tracked");
    assert!(error.missing(), "{error}");
    assert!(error.message.contains("ectropy.toml"), "{error}");
    assert!(error.message.contains("git rm ectropy.toml"), "{error}");
    assert!(error.message.contains("git add -u"), "{error}");
    assert!(error.message.contains("restore"), "{error}");
    assert!(!error.message.contains("os error 2"), "{error}");

    let staged = Snapshot::staged(root).expect("staged snapshot reads git index");
    assert_eq!(staged.entries().len(), 1);
    assert_eq!(staged.entries()[0].path(), "ectropy.toml");
    assert_eq!(staged.entries()[0].bytes(), b"policy = 1\n");
}

#[test]
fn multiple() {
    let fixture = repo();
    let root = fixture.path();
    std::fs::write(root.join("ectropy.toml"), "a = 1\n").expect("ectropy");
    std::fs::write(root.join("runseal.toml"), "b = 2\n").expect("runseal");
    std::fs::write(root.join("kept.toml"), "c = 3\n").expect("kept");
    git(root, &["add", "."]);
    std::fs::remove_file(root.join("ectropy.toml")).expect("delete ectropy");
    std::fs::remove_file(root.join("runseal.toml")).expect("delete runseal");

    let error = Snapshot::read(root).expect_err("multiple unstaged deletions must refuse");
    assert_eq!(error.kind, "tracked");
    assert!(error.missing(), "{error}");
    assert!(error.message.contains("ectropy.toml"), "{error}");
    assert!(error.message.contains("runseal.toml"), "{error}");
    assert!(!error.message.contains("kept.toml"), "{error}");
    assert!(error.message.contains("git rm"), "{error}");
    assert!(error.message.contains("git add -u"), "{error}");
    assert!(error.message.contains("restore"), "{error}");
    assert!(!error.message.contains("os error 2"), "{error}");

    git(root, &["add", "-u"]);
    let after = Snapshot::read(root).expect("staged deletions allow read");
    assert_eq!(after.entries().len(), 1);
    assert_eq!(after.entries()[0].path(), "kept.toml");
}

#[test]
fn capped() {
    let fixture = repo();
    let root = fixture.path();
    for index in 1..=12 {
        std::fs::write(root.join(format!("item{index:02}.toml")), "x\n").expect("write");
    }
    git(root, &["add", "."]);
    for index in 1..=12 {
        std::fs::remove_file(root.join(format!("item{index:02}.toml"))).expect("delete");
    }

    let error = Snapshot::read(root).expect_err("many unstaged deletions must refuse");
    assert_eq!(error.kind, "tracked");
    assert!(error.missing(), "{error}");
    assert!(error.message.contains("item01.toml"), "{error}");
    assert!(error.message.contains("item10.toml"), "{error}");
    assert!(!error.message.contains("item11.toml"), "{error}");
    assert!(error.message.contains("(and 2 more)"), "{error}");
}

#[cfg(unix)]
#[test]
fn dangling() {
    use std::os::unix::fs::symlink;

    let fixture = repo();
    let root = fixture.path();
    symlink("target", root.join("link")).expect("symlink");
    git(root, &["add", "link"]);
    std::fs::remove_file(root.join("link")).expect("delete link");

    let error = Snapshot::read(root).expect_err("deleted symlink must refuse");
    assert_eq!(error.kind, "tracked");
    assert!(error.missing(), "{error}");
    assert!(error.message.contains("link"), "{error}");

    git(root, &["add", "-u"]);
    std::fs::write(root.join("regular"), "held\n").expect("regular");
    git(root, &["add", "regular"]);
    std::fs::remove_file(root.join("regular")).expect("remove regular");
    symlink("nowhere", root.join("regular")).expect("broken symlink");

    let broken = Snapshot::read(root).expect_err("broken symlink must refuse");
    assert_eq!(broken.kind, "tracked");
    assert!(!broken.missing(), "{broken}");
    assert!(
        broken.message.contains("cannot read tracked path regular"),
        "{broken}"
    );
}
