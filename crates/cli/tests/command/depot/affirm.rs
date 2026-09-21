use super::support;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct World {
    root: tempfile::TempDir,
    media: tempfile::TempDir,
    home: tempfile::TempDir,
    profile: String,
}

impl World {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let media = tempfile::tempdir().unwrap();
        let manifest = "[[layout.seat]]\npath='.plumb'\n[[layout.seat]]\npath='skills/*'\nrule=['rule://seat/skill']\n[[layout.file]]\nname=['AGENTS.md']\nrule=['rule://seat/affirmed']\n[[layout.file]]\nname=['plumb.toml','ectropy.toml']\n";
        let profile = format!(
            "schema='plumb.product-profile/v1'\n[product]\nname='probe'\nauthority='https://releases.probe.perish.uk'\nderivatives=['configuration']\n[governance]\nmanifest={manifest:?}\nectropy='[comment]'\n"
        );
        let digest = plumb::depot::sha(profile.as_bytes());
        write(media.path(), &format!("profiles/{digest}.toml"), &profile);
        write(
            media.path(),
            "rules/products.toml",
            &format!(
                "schema='plumb.products/v2'\n[[product]]\nidentity='git.perish.top/PerishLab/probe'\nprofile='{digest}'\n"
            ),
        );
        write(
            media.path(),
            "rules/seat.toml",
            "[[member.entry]]\nname='affirmed'\naffirms=['declaration','seat','lane']\n[[member.entry]]\nname='skill'\nleaf='SKILL.md'\naffirms=['declaration','seat','lane']\n",
        );
        write(root.path(), "AGENTS.md", "# Operations\n");
        write(root.path(), "skills/probe/SKILL.md", "# Caller\n");
        write(root.path(), "plumb.toml", manifest);
        write(root.path(), "ectropy.toml", "[comment]");
        write(
            root.path(),
            ".plumb/affirmed.toml",
            "[[record]]\ntarget='AGENTS.md'\nauthority='stale'\n",
        );
        for args in [
            vec!["init", "-q"],
            vec![
                "remote",
                "add",
                "origin",
                "https://git.perish.top/PerishLab/probe.git",
            ],
            vec!["add", "."],
        ] {
            assert!(
                Command::new("git")
                    .arg("-C")
                    .arg(root.path())
                    .args(args)
                    .status()
                    .unwrap()
                    .success()
            );
        }
        let bodies = plumb::depot::v3::Bundle::contents(media.path()).unwrap();
        let texts = bodies
            .iter()
            .map(|(path, bytes)| (path.as_str(), std::str::from_utf8(bytes).unwrap()))
            .collect::<Vec<_>>();
        let home = support::depot(&texts);
        drop(bodies);
        Self {
            root,
            media,
            home,
            profile: digest,
        }
    }

    fn affirm(&self, write: bool) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_plumb"));
        command
            .current_dir(self.root.path())
            .args(["affirm", "--configuration"])
            .arg(self.media.path())
            .env("PLUMB_HOME", self.home.path());
        if write {
            command.arg("--write");
        }
        command.output().unwrap()
    }

    fn confirm(&self) -> PathBuf {
        let output = self.affirm(true);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let prefix = self
            .media
            .path()
            .join("profiles/affirmed")
            .join(&self.profile);
        std::fs::read_dir(prefix)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path()
    }

    fn verdict(&self) -> Vec<serde_json::Value> {
        let bodies = plumb::depot::v3::Bundle::contents(self.media.path()).unwrap();
        let texts = bodies
            .iter()
            .map(|(path, bytes)| (path.as_str(), std::str::from_utf8(bytes).unwrap()))
            .collect::<Vec<_>>();
        let home = support::depot(&texts);
        let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
            .current_dir(self.root.path())
            .env("PLUMB_HOME", home.path())
            .args(["doctor", ".", "--json"])
            .output()
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|error| panic!("{error}: {}", String::from_utf8_lossy(&output.stderr)));
        body["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|finding| finding["code"] == "structure.seat-affirmed")
            .cloned()
            .collect()
    }
}

fn write(root: &Path, path: &str, body: &str) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
}

#[test]
fn explicit() {
    let world = World::new();
    let output = world.affirm(false);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("plumb.affirmation/v1"));
    assert!(!world.media.path().join("profiles/affirmed").exists());
    let before = std::fs::read(world.root.path().join(".plumb/affirmed.toml")).unwrap();
    let path = world.confirm();
    let receipt = std::fs::read(&path).unwrap();
    assert_eq!(
        path.file_stem().unwrap().to_str().unwrap(),
        plumb::depot::sha(&receipt)
    );
    assert_eq!(world.confirm(), path);
    assert_eq!(
        std::fs::read(world.root.path().join(".plumb/affirmed.toml")).unwrap(),
        before
    );
    assert!(world.verdict().is_empty(), "{:#?}", world.verdict());
}

#[test]
fn drift() {
    let world = World::new();
    assert!(!world.verdict().is_empty());
    world.confirm();
    assert!(world.verdict().is_empty(), "{:#?}", world.verdict());
    write(world.root.path(), "AGENTS.md", "# Changed operations\n");
    assert!(!world.verdict().is_empty());
    world.confirm();
    assert!(world.verdict().is_empty(), "{:#?}", world.verdict());
    write(
        world.root.path(),
        "skills/probe/SKILL.md",
        "# Changed caller\n",
    );
    assert!(!world.verdict().is_empty());
}

#[test]
fn rules() {
    let world = World::new();
    world.confirm();
    let path = world.media.path().join("rules/seat.toml");
    let old = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        format!("{old}\n[[member.entry]]\nname='unrelated'\n"),
    )
    .unwrap();
    assert!(world.verdict().is_empty(), "{:#?}", world.verdict());
    std::fs::write(
        &path,
        old.replace("name='affirmed'", "name='affirmed'\nbytes=999"),
    )
    .unwrap();
    assert!(!world.verdict().is_empty());
}

#[test]
fn tampered() {
    let world = World::new();
    let path = world.confirm();
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, format!("{text}\n")).unwrap();
    let findings = world.verdict();
    assert!(
        findings.iter().any(|finding| finding["grade"] == "blind"),
        "{findings:?}"
    );
}

#[test]
#[cfg(unix)]
fn symbolic() {
    let world = World::new();
    let outside = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(outside.path(), world.media.path().join("profiles/affirmed"))
        .unwrap();
    let output = world.affirm(true);
    assert!(!output.status.success());
    assert_eq!(std::fs::read_dir(outside.path()).unwrap().count(), 0);
    let world = World::new();
    let source = world.root.path().join("AGENTS.md");
    std::fs::rename(&source, outside.path().join("AGENTS.md")).unwrap();
    std::os::unix::fs::symlink(outside.path().join("AGENTS.md"), source).unwrap();
    assert!(!world.affirm(true).status.success());
    let world = World::new();
    std::fs::rename(
        world.root.path().join("skills"),
        outside.path().join("skills"),
    )
    .unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("skills"),
        world.root.path().join("skills"),
    )
    .unwrap();
    assert!(!world.affirm(true).status.success());
}
