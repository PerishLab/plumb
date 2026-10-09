use super::precommit::Repo;
use super::precommit::cache::{fixture, run, seats, success};
use super::support;
use serde_json::Value;
use std::path::{Path, PathBuf};

const AGED: u64 = 8 * 24 * 60 * 60;

fn reclaim(home: &Path, apply: bool) -> Value {
    let mut command = support::plumb();
    command.args(["cache", "reclaim", "--json"]);
    if apply {
        command.arg("--apply");
    }
    let output = command.env("PLUMB_HOME", home).output().expect("reclaim");
    success(&output);
    serde_json::from_slice(&output.stdout).expect("reclaim report")
}

fn entry(report: &Value, bucket: &Path) -> Value {
    let name = bucket.file_name().expect("bucket name").to_string_lossy();
    report["buckets"]
        .as_array()
        .expect("buckets")
        .iter()
        .find(|entry| entry["bucket"] == name.as_ref())
        .cloned()
        .expect("bucket entry")
}

fn ledger(home: &Path, bucket: &Path) -> PathBuf {
    let name = bucket.file_name().expect("bucket name").to_string_lossy();
    home.join("state/guard/cargo").join(format!("{name}.json"))
}

fn age(home: &Path, bucket: &Path, keys: Option<&[&str]>) {
    let path = ledger(home, bucket);
    let mut record: Value =
        serde_json::from_slice(&std::fs::read(&path).expect("record")).expect("record json");
    let units = record["units"].as_object_mut().expect("units");
    for (key, used) in units.iter_mut() {
        if keys.is_none_or(|keys| keys.contains(&key.as_str())) {
            *used = (used.as_u64().expect("stamp") - AGED).into();
        }
    }
    if keys.is_none() {
        record["used"] = (record["used"].as_u64().expect("used") - AGED).into();
    }
    std::fs::write(&path, serde_json::to_vec(&record).expect("record bytes")).expect("aged");
}

fn inode(path: &Path) -> u64 {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata(path).expect("lease").ino()
}

fn change(root: &Path, answer: u8) {
    std::fs::write(
        root.join("src/lib.rs"),
        format!("pub fn answer() -> u8 {{\n    {answer}\n}}\n"),
    )
    .expect("changed source");
    Repo::git(root, &["add", "src/lib.rs"]);
}

#[test]
fn idle() {
    let fixture = fixture();
    let home = support::home();
    success(&run(fixture.path(), home.path()));
    let bucket = seats(home.path()).remove(0);
    assert!(bucket.join("plumb.json").is_file());
    let lease = inode(&bucket.join("lease"));
    let fresh = reclaim(home.path(), false);
    assert_eq!(entry(&fresh, &bucket)["status"], "live");
    age(home.path(), &bucket, None);
    let preview = entry(&reclaim(home.path(), false), &bucket);
    assert_eq!(preview["status"], "idle");
    assert!(preview["reclaimable"].as_u64().expect("bytes") > 0);
    assert!(bucket.join("debug").is_dir());
    let applied = entry(&reclaim(home.path(), true), &bucket);
    assert_eq!(applied["reclaimed"], applied["reclaimable"]);
    assert!(!bucket.join("debug").exists());
    assert!(bucket.join("plumb.json").is_file());
    assert_eq!(inode(&bucket.join("lease")), lease);
    change(fixture.path(), 45);
    let rebuilt = run(fixture.path(), home.path());
    success(&rebuilt);
    assert!(String::from_utf8_lossy(&rebuilt.stderr).contains("guard guard/rust"));
    assert!(bucket.join("debug/deps").is_dir());
    assert_eq!(
        entry(&reclaim(home.path(), false), &bucket)["status"],
        "live"
    );
}

#[test]
fn held() {
    let fixture = fixture();
    let home = support::home();
    success(&run(fixture.path(), home.path()));
    let bucket = seats(home.path()).remove(0);
    age(home.path(), &bucket, None);
    let lock = std::fs::File::open(bucket.join("lease")).expect("lease");
    lock.lock().expect("held lease");
    let refused = entry(&reclaim(home.path(), true), &bucket);
    assert_eq!(refused["status"], "active");
    assert_eq!(refused["reclaimed"], 0);
    assert!(bucket.join("debug").is_dir());
    drop(lock);
    let applied = entry(&reclaim(home.path(), true), &bucket);
    assert_eq!(applied["status"], "idle");
    assert!(!bucket.join("debug").exists());
}

#[test]
fn abandoned() {
    let fixture = fixture();
    let home = support::home();
    success(&run(fixture.path(), home.path()));
    let bucket = seats(home.path()).remove(0);
    drop(fixture);
    let applied = entry(&reclaim(home.path(), true), &bucket);
    assert_eq!(applied["status"], "abandoned");
    assert!(applied["reclaimed"].as_u64().expect("bytes") > 0);
    assert!(!bucket.join("debug").exists());
    assert!(bucket.join("lease").is_file());
}

#[test]
fn units() {
    let fixture = fixture();
    let home = support::home();
    success(&run(fixture.path(), home.path()));
    let bucket = seats(home.path()).remove(0);
    let ghost = bucket.join("debug/deps/libghost-0123456789abcdef.rlib");
    std::fs::write(&ghost, "interrupted output").expect("orphan");
    let record: Value =
        serde_json::from_slice(&std::fs::read(ledger(home.path(), &bucket)).expect("record"))
            .expect("record json");
    let stale = record["units"]
        .as_object()
        .expect("units")
        .keys()
        .find(|key| key.starts_with("debug/probe-"))
        .cloned()
        .expect("probe unit");
    age(home.path(), &bucket, Some(&[stale.as_str()]));
    let applied = entry(&reclaim(home.path(), true), &bucket);
    assert_eq!(applied["status"], "live");
    let targets: Vec<&str> = applied["targets"]
        .as_array()
        .expect("targets")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(targets.contains(&"debug/deps/libghost-0123456789abcdef.rlib"));
    assert!(
        targets
            .iter()
            .any(|target| target.starts_with("debug/.fingerprint/probe-"))
    );
    assert!(!ghost.exists());
    assert!(
        !bucket
            .join("debug/.fingerprint")
            .join(stale.trim_start_matches("debug/"))
            .exists()
    );
    change(fixture.path(), 46);
    success(&run(fixture.path(), home.path()));
}

#[test]
fn unknown() {
    let home = support::home();
    let fixture = fixture();
    success(&run(fixture.path(), home.path()));
    let stranger = seats(home.path())[0].with_file_name("legacy");
    std::fs::create_dir_all(stranger.join("debug/deps")).expect("legacy bucket");
    std::fs::write(
        stranger.join("debug/deps/libold-0123456789abcdef.rlib"),
        "kept",
    )
    .expect("legacy unit");
    let applied = entry(&reclaim(home.path(), true), &stranger);
    assert_eq!(applied["status"], "unknown");
    assert_eq!(applied["reclaimed"], 0);
    assert!(
        stranger
            .join("debug/deps/libold-0123456789abcdef.rlib")
            .is_file()
    );
}
