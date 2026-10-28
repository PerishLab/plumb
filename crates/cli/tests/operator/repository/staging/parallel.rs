use super::{Gated, Repo, cache, registered, support};
use std::path::Path;
use std::process::{Child, Stdio};

fn spawn(root: &Path, home: &Path) -> Child {
    support::plumb()
        .args(["guard", ".", "--json"])
        .current_dir(root)
        .env("PLUMB_HOME", home)
        .env("PLUMB_GUARD_CACHE_RESERVE", "0")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

fn waiting(guard: &mut Child) {
    let waiter = format!(" {} ", guard.id());
    for _ in 0..600 {
        assert!(
            guard.try_wait().unwrap().is_none(),
            "Guard escaped its source lease"
        );
        if std::fs::read_to_string("/proc/locks")
            .unwrap()
            .lines()
            .any(|line| line.contains("->") && line.contains(&waiter))
        {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    panic!("Guard never waited on its lease");
}

#[test]
fn shared() {
    let gated = Gated::new();
    Repo::git(gated.root(), &["commit", "-qm", "shared base"]);
    let home = support::home();
    let sibling = tempfile::tempdir().unwrap();
    let other = sibling.path().join("other");
    Repo::git(
        gated.root(),
        &[
            "worktree",
            "add",
            "--detach",
            "--quiet",
            other.to_str().unwrap(),
        ],
    );
    std::fs::write(
        other.join("src/lib.rs"),
        "pub fn answer() -> u8 {\n    43\n}\n",
    )
    .unwrap();
    Repo::git(&other, &["add", "src/lib.rs"]);
    let first = gated.spawn(home.path(), false);
    let mut second = spawn(&other, home.path());
    waiting(&mut second);
    gated.open();
    let first = first.wait_with_output().unwrap();
    let second = second.wait_with_output().unwrap();
    cache::success(&first);
    cache::success(&second);
    for (root, output) in [(gated.root(), first), (other.as_path(), second)] {
        let proof: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let tree = Repo::git(root, &["write-tree"]);
        assert_eq!(proof["tree"], String::from_utf8_lossy(&tree.stdout).trim());
    }
    assert_eq!(
        registered(gated.root()).len(),
        2,
        "one sibling and one retained source"
    );
    let source = super::source::seat(home.path());
    let lease = std::fs::File::open(source.with_extension("lock")).unwrap();
    radicula_lock::lock(&lease, radicula_lock::Mode::Exclusive).unwrap();
    let mut reused = spawn(&other, home.path());
    waiting(&mut reused);
    radicula_lock::unlock(&lease).unwrap();
    cache::success(&reused.wait_with_output().unwrap());
}
