use super::{Ledger, change, entry, inode, sweep};
use crate::precommit::cache::{fixture, run, seats, success};
use crate::support;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Output;

fn guarded(root: &Path, home: &Path, reserve: &str) -> Output {
    support::plumb()
        .args(["guard", ".", "--json"])
        .current_dir(root)
        .env("PLUMB_HOME", home)
        .env("PLUMB_GUARD_CACHE_RESERVE", reserve)
        .env("CARGO_TARGET_DIR", root.join("unowned"))
        .output()
        .expect("guard")
}

fn status(home: &Path) -> Output {
    support::plumb()
        .args(["cache", "status", "--json"])
        .env("PLUMB_HOME", home)
        .env_remove("PLUMB_GUARD_CACHE_RESERVE")
        .output()
        .expect("status")
}

fn disk(output: &Output) -> Value {
    success(output);
    let report: Value = serde_json::from_slice(&output.stdout).expect("status report");
    report["disk"].clone()
}

fn bucket(home: &Path, root: &Path) -> PathBuf {
    let common = std::fs::canonicalize(root.join(".git")).expect("common directory");
    seats(home)
        .into_iter()
        .find(|seat| {
            std::fs::read_to_string(seat.join("plumb.json"))
                .is_ok_and(|marker| marker.contains(common.to_string_lossy().as_ref()))
        })
        .expect("bucket of repository")
}

#[test]
fn whole() {
    let first = fixture();
    let second = fixture();
    let home = support::home();
    success(&run(first.path(), home.path()));
    change(second.path(), 44);
    success(&run(second.path(), home.path()));
    let old = bucket(home.path(), first.path());
    let recent = bucket(home.path(), second.path());
    assert_ne!(old, recent);
    Ledger::of(home.path(), &old).shift(None, 60);
    let lease = inode(&old.join("lease"));
    let calm = sweep(home.path(), false, "0");
    assert_eq!(entry(&calm, &old)["evicted"], false);
    assert_eq!(entry(&calm, &recent)["evicted"], false);
    let preview = sweep(home.path(), false, "100");
    let held = entry(&preview, &old);
    assert_eq!(held["status"], "live");
    assert_eq!(held["evicted"], true);
    assert!(
        held["reclaimable"].as_u64().expect("bytes") >= held["working"].as_u64().expect("working")
    );
    assert!(held["working"].as_u64().expect("working") > 0);
    assert_eq!(entry(&preview, &recent)["evicted"], false);
    assert!(old.join("debug").is_dir());
    let applied = sweep(home.path(), true, "100");
    let evicted = entry(&applied, &old);
    assert_eq!(evicted["evicted"], true);
    assert_eq!(evicted["reclaimed"], evicted["reclaimable"]);
    assert!(!old.join("debug").exists());
    assert!(old.join("plumb.json").is_file());
    assert_eq!(inode(&old.join("lease")), lease);
    assert_eq!(entry(&applied, &recent)["evicted"], false);
    assert!(recent.join("debug/deps").is_dir());
    change(first.path(), 45);
    let rebuilt = run(first.path(), home.path());
    success(&rebuilt);
    assert!(String::from_utf8_lossy(&rebuilt.stderr).contains("guard guard/rust"));
    assert!(old.join("debug/deps").is_dir());
    change(second.path(), 46);
    success(&guarded(second.path(), home.path(), "100"));
    assert!(!old.join("debug").exists());
    assert!(recent.join("debug/deps").is_dir());
}

#[test]
fn configured() {
    let fixture = fixture();
    let home = support::home();
    success(&run(fixture.path(), home.path()));
    let file = home.path().join("plumb.toml");
    assert_eq!(disk(&status(home.path()))["reserve"], 30);
    std::fs::write(
        &file,
        "[locus]\nenabled = false\n\n[cache]\nreserve = 100\n",
    )
    .expect("config");
    let declared = disk(&status(home.path()));
    assert_eq!(declared["reserve"], 100);
    assert_eq!(declared["pressure"], true);
    assert_eq!(declared["line"], declared["total"]);
    assert!(declared["deficit"].as_u64().expect("deficit") > 0);
    let overridden = sweep(home.path(), false, "0")["disk"].clone();
    assert_eq!(overridden["reserve"], 0);
    assert_eq!(overridden["pressure"], false);
    assert_eq!(overridden["deficit"], 0);
    std::fs::write(&file, "[cache]\nreserve = 101\n").expect("config");
    let refused = status(home.path());
    assert!(!refused.status.success());
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(stderr.contains("[cache] reserve"), "{stderr}");
    assert!(stderr.contains("101"), "{stderr}");
}
