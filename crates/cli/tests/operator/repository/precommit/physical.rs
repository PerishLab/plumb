#![cfg(unix)]

use super::{Repo, cache, support};

#[test]
#[cfg(unix)]
fn ancestor() {
    let fixture = cache::fixture();
    let root = fixture.path();
    let home = support::home();
    let physical = tempfile::tempdir().expect("physical seat");
    let target = physical.path().join("parent/seat");
    std::fs::create_dir_all(&target).expect("physical parent");
    std::os::unix::fs::symlink(&target, home.path().join("tmp")).expect("seat alias");
    cache::success(&cache::run(root, home.path()));
    let configuration = physical.path().join("parent/.cargo");
    std::fs::create_dir(&configuration).expect("host configuration");
    std::fs::write(
        configuration.join("config.toml"),
        "[build]\nrustflags = ['--invalid-probe']\n",
    )
    .expect("host override");
    for changed in [false, true] {
        if changed {
            std::fs::write(root.join("NOTES"), "unrelated\n").expect("unrelated");
            Repo::git(root, &["add", "NOTES"]);
        }
        let output = cache::run(root, home.path());
        assert!(
            !output.status.success(),
            "physical ancestor must not reuse proof"
        );
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("unbound host configuration"), "{error}");
        assert!(!error.contains("guard guard/rust"), "{error}");
    }
}

#[test]
#[cfg(unix)]
fn dangling() {
    let fixture = cache::fixture();
    let home = support::home();
    let physical = tempfile::tempdir().expect("physical seat");
    std::os::unix::fs::symlink(physical.path().join("absent"), home.path().join("tmp"))
        .expect("broken alias");
    let output = cache::run(fixture.path(), home.path());
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        error.contains(&format!(
            "cannot create {}",
            home.path().join("tmp").display()
        )),
        "{error}"
    );
    assert!(!physical.path().join("absent").exists());
    assert_eq!(
        std::fs::read_link(home.path().join("tmp")).unwrap(),
        physical.path().join("absent")
    );
    assert!(!error.contains("guard guard/rust"), "{error}");
}

#[test]
#[cfg(target_os = "linux")]
fn waiting() {
    let fixture = tempfile::tempdir().expect("repository");
    let root = fixture.path().join("repository");
    for (path, body) in [
        (
            "Cargo.toml",
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        ),
        (
            "src/main.rs",
            "fn main() {\n    print!(\"{}\", include_str!(\"../help.txt\"));\n}\n",
        ),
        (
            "tests/closure.rs",
            "#[test]\nfn command() {\n    let out = std::process::Command::new(env!(\"CARGO_BIN_EXE_demo\")).output().unwrap();\n    assert_eq!(String::from_utf8(out.stdout).unwrap(), \"old help\\n\");\n}\n",
        ),
        ("help.txt", "old help\n"),
        ("plumb.toml", "[workflow.hash.guard]\ntest = [\"*\"]\n"),
    ] {
        std::fs::create_dir_all(root.join(path).parent().expect("parent")).expect("seat");
        std::fs::write(root.join(path), body).expect("source");
    }
    let output = std::process::Command::new("cargo")
        .args(["generate-lockfile", "--offline"])
        .current_dir(&root)
        .output()
        .expect("lock");
    assert!(output.status.success());
    Repo::git(&root, &["init", "-q", "-b", "main"]);
    Repo::git(
        &root,
        &[
            "remote",
            "add",
            "origin",
            "https://git.example.invalid/Example/demo.git",
        ],
    );
    Repo::git(&root, &["add", "-A"]);
    Repo::git(
        &root,
        &[
            "-c",
            "user.name=Plumb Test",
            "-c",
            "user.email=plumb@example.invalid",
            "commit",
            "-qm",
            "base",
        ],
    );
    let (other, own) = (fixture.path().join("other"), fixture.path().join("own"));
    for tree in [&other, &own] {
        Repo::git(
            &root,
            &[
                "worktree",
                "add",
                "-q",
                "--detach",
                tree.to_str().expect("path"),
            ],
        );
    }
    std::fs::write(other.join("help.txt"), "new help\n").expect("other help");
    Repo::git(&other, &["add", "help.txt"]);
    std::fs::write(own.join("NOTES"), "unrelated\n").expect("own note");
    Repo::git(&own, &["add", "NOTES"]);
    let home = support::home();
    let seeded = cache::run(&other, home.path());
    assert!(String::from_utf8_lossy(&seeded.stderr).contains("left: \"new help\\n\""));
    let [seat] = cache::seats(home.path()).try_into().expect("one bucket");
    let lease = std::fs::File::options()
        .read(true)
        .write(true)
        .open(seat.join("lease"))
        .expect("lease");
    lease.lock().expect("held lease");
    let mut guard = spawned(&own, home.path());
    assert!(blocked(&mut guard), "guard finished under a held lease");
    touch(&seat);
    lease.unlock().expect("released lease");
    cache::success(&guard.wait_with_output().expect("waited guard"));
    lease.lock().expect("held lease");
    let mut proved = spawned(&own, home.path());
    assert!(!blocked(&mut proved), "a proved tree waited on the lease");
    cache::success(&proved.wait_with_output().expect("proved guard"));
}

#[cfg(target_os = "linux")]
fn spawned(root: &std::path::Path, home: &std::path::Path) -> std::process::Child {
    support::plumb()
        .args(["guard", ".", "--json"])
        .current_dir(root)
        .env("PLUMB_HOME", home)
        .env("PLUMB_GUARD_CACHE_RESERVE", "0")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("guard")
}

#[cfg(target_os = "linux")]
fn blocked(guard: &mut std::process::Child) -> bool {
    let waiter = format!(" {} ", guard.id());
    for _ in 0..1200 {
        let locks = std::fs::read_to_string("/proc/locks").expect("locks");
        if locks
            .lines()
            .any(|line| line.contains("->") && line.contains(&waiter))
        {
            return true;
        }
        if guard.try_wait().expect("guard state").is_some() {
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    panic!("guard neither finished nor waited on the bucket lease");
}

#[cfg(target_os = "linux")]
fn touch(path: &std::path::Path) {
    for entry in std::fs::read_dir(path).expect("bucket") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            touch(&path);
        } else {
            std::fs::File::options()
                .append(true)
                .open(&path)
                .and_then(|file| file.set_modified(std::time::SystemTime::now()))
                .expect("newer output");
        }
    }
}
