use super::{Ledger, change, entry, inode, reclaim, sweep};
use crate::precommit::Repo;
use crate::precommit::cache::{fixture, run, seats, success};
use crate::support;
use std::path::{Path, PathBuf};

fn fingerprint(bucket: &Path, key: &str) -> PathBuf {
    bucket
        .join("debug/.fingerprint")
        .join(key.trim_start_matches("debug/"))
}

fn feature(root: &Path, name: &str) {
    std::fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"probe\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[features]\ndefault = [\"{name}\"]\n{name} = []\n"
        ),
    )
    .expect("manifest");
    let output = std::process::Command::new("cargo")
        .args(["generate-lockfile", "--offline"])
        .current_dir(root)
        .output()
        .expect("lock");
    assert!(output.status.success());
    Repo::git(root, &["add", "Cargo.toml", "Cargo.lock"]);
}

fn backdate(ledger: &Ledger, bucket: &Path) {
    fn walk(path: &Path, when: std::time::SystemTime) {
        for entry in std::fs::read_dir(path).expect("listing") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                walk(&path, when);
                std::fs::File::open(&path)
                    .and_then(|file| file.set_modified(when))
                    .expect("backdated directory");
            } else {
                std::fs::File::options()
                    .write(true)
                    .open(&path)
                    .and_then(|file| file.set_modified(when))
                    .expect("backdated");
            }
        }
    }
    let when = std::time::SystemTime::now() - std::time::Duration::from_secs(60);
    walk(&bucket.join("debug"), when);
    walk(&bucket.join("release"), when);
    ledger.shift(None, 60);
}

#[test]
fn bounded() {
    let fixture = fixture();
    let home = support::home();
    success(&run(fixture.path(), home.path()));
    let bucket = seats(home.path()).remove(0);
    let ledger = Ledger::of(home.path(), &bucket);
    let first = ledger.probes();
    assert!(!first.is_empty());
    for name in ["first", "second"] {
        backdate(&ledger, &bucket);
        feature(fixture.path(), name);
        success(&run(fixture.path(), home.path()));
    }
    for key in &first {
        assert!(
            !fingerprint(&bucket, key).exists(),
            "{key} outlived the bound"
        );
    }
    let held = entry(&reclaim(home.path(), false), &bucket);
    assert_eq!(held["status"], "live");
    let working = held["working"].as_u64().expect("working");
    assert!(working > 0);
    assert!(held["superseded"].as_u64().expect("superseded") <= working);
    for key in ledger.probes() {
        assert!(!first.contains(&key));
    }
}

#[test]
fn pressure() {
    let fixture = fixture();
    let home = support::home();
    success(&run(fixture.path(), home.path()));
    let bucket = seats(home.path()).remove(0);
    let lease = inode(&bucket.join("lease"));
    let ledger = Ledger::of(home.path(), &bucket);
    let held = ledger.probes();
    let (old, kept) = held.split_at(1);
    ledger.shift(
        Some(&old.iter().map(String::as_str).collect::<Vec<_>>()),
        60,
    );
    let calm = entry(&sweep(home.path(), false, "0"), &bucket);
    let superseded = calm["superseded"].as_u64().expect("superseded");
    assert!(superseded > 0);
    let preview = entry(&sweep(home.path(), false, "100"), &bucket);
    assert_eq!(preview["status"], "live");
    assert!(preview["reclaimable"].as_u64().expect("bytes") >= superseded);
    assert!(fingerprint(&bucket, &old[0]).exists());
    let applied = entry(&sweep(home.path(), true, "100"), &bucket);
    assert_eq!(applied["reclaimed"], applied["reclaimable"]);
    assert!(!fingerprint(&bucket, &old[0]).exists());
    for key in kept {
        assert!(fingerprint(&bucket, key).exists());
    }
    assert_eq!(
        entry(&sweep(home.path(), false, "100"), &bucket)["superseded"],
        0
    );
    assert_eq!(inode(&bucket.join("lease")), lease);
    change(fixture.path(), 47);
    let rebuilt = run(fixture.path(), home.path());
    success(&rebuilt);
    assert!(String::from_utf8_lossy(&rebuilt.stderr).contains("guard guard/rust"));
}
