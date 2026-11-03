use super::runtime::Fixture;
use radicula_lock::Mode;
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

fn file(root: &Path) -> File {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(root.join("lease"))
        .unwrap()
}

fn awaiter(root: &Path, name: &str) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !root.join(name).exists() {
        assert!(Instant::now() < deadline, "missing fixture marker {name}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

struct Owner<'a> {
    child: Child,
    fixture: &'a Fixture,
}

impl Drop for Owner<'_> {
    fn drop(&mut self) {
        for name in ["a", "b", "c"] {
            self.fixture.write(&format!("{name}.release"), "release");
        }
        self.child.wait().unwrap();
    }
}

fn child(root: &Path) {
    super::super::signal::watch();
    let held = super::super::signal::Hold::new().unwrap();
    let lease = file(root);
    radicula_lock::lock(&lease, Mode::Exclusive).unwrap();
    let carrier = Fixture::carrier();
    let runner = super::Runner {
        carrier: &carrier,
        command: &|| Fixture::command(root),
        cores: 2,
    };
    assert!(runner.execute(root).is_err());
    assert!(root.join("a.end").exists());
    assert!(root.join("b.end").exists());
    assert!(!root.join("c.start").exists());
    drop(lease);
    std::fs::write(root.join("reaped"), "done").unwrap();
    let code = super::super::signal::received().unwrap_or(0);
    drop(held);
    std::process::exit(code);
}

fn scenario(signal: Option<&str>) {
    let fixture = Fixture::new();
    fixture.controlled();
    let child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "execution::suite::cancel::lifetime",
            "--nocapture",
        ])
        .env("PLUMB_SUITE_CHILD", fixture.root())
        .spawn()
        .unwrap();
    let mut owner = Owner {
        child,
        fixture: &fixture,
    };
    awaiter(fixture.root(), "a.start");
    awaiter(fixture.root(), "b.start");
    let lease = file(fixture.root());
    assert!(radicula_lock::attempt(&lease, Mode::Exclusive).is_err());
    if let Some(signal) = signal {
        assert!(
            Command::new("kill")
                .args([signal, &owner.child.id().to_string()])
                .status()
                .unwrap()
                .success()
        );
    } else {
        fixture.write("a.fail", "fail");
    }
    fixture.write("a.release", "release");
    awaiter(fixture.root(), "a.end");
    std::thread::sleep(Duration::from_millis(100));
    assert!(owner.child.try_wait().unwrap().is_none());
    assert!(radicula_lock::attempt(&lease, Mode::Exclusive).is_err());
    assert!(!fixture.root().join("c.start").exists());
    fixture.write("b.release", "release");
    let status = owner.child.wait().unwrap();
    let expected = match signal {
        Some("-INT") => 130,
        Some("-TERM") => 143,
        _ => 0,
    };
    assert_eq!(status.code(), Some(expected));
    assert!(fixture.root().join("reaped").exists());
    radicula_lock::attempt(&lease, Mode::Exclusive).unwrap();
}

#[test]
fn lifetime() {
    if let Some(root) = std::env::var_os("PLUMB_SUITE_CHILD") {
        child(Path::new(&root));
        return;
    }
    for signal in [None, Some("-INT"), Some("-TERM")] {
        scenario(signal);
    }
}
