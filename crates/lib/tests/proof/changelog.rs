use plumb::changelog::{Claim, prove};
use std::path::Path;
use std::process::Command;

struct Repo<'a>(&'a Path);

#[test]
fn measured() {
    let fixture = tempfile::tempdir().expect("fixture");
    let seat = tempfile::tempdir().expect("staged");
    let root = fixture.path();
    let repo = Repo(root);
    repo.init();
    std::fs::write(root.join("prior.rs"), "old\n").expect("prior");
    let base = repo.commit("base");
    std::fs::write(root.join("source.rs"), "line\n").expect("source");
    let candidate = repo.commit("candidate");
    staged(seat.path(), "note\n");
    let proof = prove(Claim {
        root,
        home: seat.path(),
        version: "1.1.0",
        previous: Some(&base),
        candidate: &candidate,
    })
    .expect("proof");
    assert_eq!(proof.base, base);
    assert_eq!(proof.candidate, candidate);
    assert_eq!(proof.units, 2);
    assert_eq!(proof.languages["en"].budget, 120);
    assert_eq!(proof.languages["zh"].lines, 2);
}

#[test]
fn bounded() {
    let fixture = tempfile::tempdir().expect("fixture");
    let held = tempfile::tempdir().expect("staged");
    let root = fixture.path();
    let repo = Repo(root);
    repo.init();
    std::fs::write(root.join("prior.rs"), "old\n").expect("prior");
    let base = repo.commit("base");
    std::fs::write(root.join("source.rs"), "line\n").expect("source");
    let candidate = repo.commit("candidate");
    staged(held.path(), &"line\n".repeat(200));
    let error = prove(Claim {
        root,
        home: held.path(),
        version: "1.1.0",
        previous: Some(&base),
        candidate: &candidate,
    })
    .expect_err("above budget");
    assert!(error.contains("above diff budget"), "{error}");
}

fn staged(home: &Path, text: &str) {
    for tongue in ["en", "zh"] {
        let seat = home.join(tongue);
        std::fs::create_dir_all(&seat).expect("changelog seat");
        std::fs::write(seat.join("INDEX.md"), text).expect("index");
        std::fs::write(seat.join("MIGRATION.md"), text).expect("migration");
    }
}

impl Repo<'_> {
    fn init(&self) {
        self.run(&["init", "-q"]);
        self.run(&["config", "user.name", "Fixture"]);
        self.run(&["config", "user.email", "fixture@example.test"]);
    }

    fn commit(&self, message: &str) -> String {
        self.run(&["add", "."]);
        self.run(&["commit", "-qm", message]);
        self.output(&["rev-parse", "HEAD"])
    }

    fn run(&self, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(self.0)
            .args(args)
            .status()
            .expect("git");
        assert!(status.success(), "git {args:?}");
    }

    fn output(&self, args: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(self.0)
            .args(args)
            .output()
            .expect("git");
        assert!(output.status.success(), "git {args:?}");
        String::from_utf8(output.stdout)
            .expect("utf8")
            .trim()
            .to_string()
    }
}
