use std::path::Path;
use std::process::Command;

struct Fixture<'a>(&'a Path);

pub fn govern(root: &Path) {
    let status = Command::new("git")
        .args([
            "-C",
            root.to_str().expect("path should be utf8"),
            "init",
            "-q",
        ])
        .status()
        .expect("git should run");
    assert!(status.success(), "fixture should become a repository");
}

pub fn run(root: &Path) -> String {
    let home = super::support::depot(&[]);
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["doctor", root.to_str().expect("path should be utf8")])
        .env_remove("PLUMB_RELEASE_VERSION")
        .env("PLUMB_HOME", home.path())
        .output()
        .expect("plumb should run");
    String::from_utf8_lossy(&output.stdout).to_string()
}

pub fn profile(product: &str, manifest: &str, ectropy: &str) -> String {
    format!(
        "schema = \"plumb.product-profile/v1\"\n\n[product]\nname = \"{product}\"\nauthority = \"https://releases.{product}.perish.uk\"\nderivatives = [\"skill\"]\n\n[governance]\nmanifest = '''\n{manifest}'''\nectropy = '''\n{ectropy}'''\n"
    )
}

#[cfg(unix)]
pub fn hooks(root: &Path) {
    use std::os::unix::fs::PermissionsExt as _;

    for name in ["pre-commit", "commit-msg"] {
        let hook = root.join(".git/hooks").join(name);
        std::fs::write(&hook, "#!/bin/sh\nexit 0\n").expect("hook");
        let mut mode = std::fs::metadata(&hook)
            .expect("hook metadata")
            .permissions();
        mode.set_mode(0o755);
        std::fs::set_permissions(&hook, mode).expect("hook mode");
    }
}

#[test]
#[cfg(unix)]
fn doctor() {
    let fixture = tempfile::tempdir().expect("fixture");
    let root = fixture.path().join("probe");
    std::fs::create_dir(&root).expect("repository");
    let repo = Fixture(&root);
    repo.git(&["init", "-q"]);
    repo.git(&[
        "remote",
        "add",
        "origin",
        "ssh://git@git.perish.top/PerishFire/probe.git",
    ]);
    std::fs::create_dir(root.join("src")).expect("source");
    std::fs::write(root.join(".gitignore"), "target/\n").expect("ignore");
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"probe\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "pub fn answer() -> u8 { 42 }\n").expect("source");
    repo.git(&["add", "-A"]);
    let tree = repo.git(&["write-tree"]);

    let document = profile(
        "probe",
        "[release]\nproduct = \"probe\"\nauthority = \"https://releases.probe.perish.uk\"\n[release.cargo]\nregistry = \"perish\"\npackages = [\"probe\"]\n\n[layout]\n[[layout.seat]]\npath = \"src\"\n[[layout.file]]\nname = [\".gitignore\", \"Cargo.toml\"]\n",
        "[comment]\nallow = false\n[limit]\nblock = 4\nfanout = 10\nfile = 300\nmarkup = 8\nparam = 4\npath = 3\n[word]\nsingle = true\n",
    );
    let digest = plumb::depot::sha(document.as_bytes());
    let catalog = format!(
        "schema = \"plumb.products/v2\"\n\n[[product]]\nidentity = \"git.perish.top/PerishFire/probe\"\nprofile = \"{digest}\"\n"
    );
    let path = format!("profiles/{digest}.toml");
    let depot = super::support::depot(&[("rules/products.toml", &catalog), (&path, &document)]);
    let missing = repo.inspect(depot.path());
    assert!(!missing.status.success());
    let report: serde_json::Value = serde_json::from_slice(&missing.stdout).expect("missing hooks");
    assert!(
        report["findings"]
            .as_array()
            .is_some_and(|findings| findings.iter().any(|finding| finding["evidence"]
                .as_str()
                .is_some_and(|held| held.contains(".git/hooks/pre-commit is absent"))))
    );

    hooks(&root);
    let output = repo.inspect(depot.path());
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("report");
    assert_eq!(report["profile"], digest);
    assert!(report["configuration"].as_str().is_some());
    assert_eq!(report["ok"], true);
    assert!(!root.join("plumb.toml").exists());
    assert!(!root.join("ectropy.toml").exists());
    assert_eq!(repo.git(&["write-tree"]), tree);
    let surface = repo.surface(depot.path());
    assert!(
        surface.status.success(),
        "{}",
        String::from_utf8_lossy(&surface.stderr)
    );
    let surface: serde_json::Value = serde_json::from_slice(&surface.stdout).expect("surface");
    assert_eq!(
        surface["publication"]["include"][0]["operation"]["type"],
        "cargo"
    );
    assert_eq!(surface["publication"]["include"][0]["profile"], digest);
    assert!(
        surface["publication"]["include"][0]["configuration"]
            .as_str()
            .is_some()
    );

    std::fs::write(root.join("plumb.toml"), "").expect("second expression");
    let refusal = repo.inspect(depot.path());
    assert!(!refusal.status.success());
    let report: serde_json::Value = serde_json::from_slice(&refusal.stdout).expect("refusal");
    assert!(
        report["findings"]
            .as_array()
            .is_some_and(|findings| findings.iter().any(|finding| finding["evidence"]
                .as_str()
                .is_some_and(|held| held.contains("must not carry plumb.toml or ectropy.toml"))))
    );

    let drifted = document.replacen("product = \"probe\"", "product = \"other\"", 1);
    let wrong = plumb::depot::sha(drifted.as_bytes());
    let catalog = format!(
        "schema = \"plumb.products/v2\"\n\n[[product]]\nidentity = \"git.perish.top/PerishFire/probe\"\nprofile = \"{wrong}\"\n"
    );
    let path = format!("profiles/{wrong}.toml");
    let depot = super::support::depot(&[("rules/products.toml", &catalog), (&path, &drifted)]);
    let refusal = repo.surface(depot.path());
    assert!(!refusal.status.success());
    assert!(
        String::from_utf8_lossy(&refusal.stderr)
            .contains("release identity differs from its product definition")
    );
}

impl Fixture<'_> {
    fn inspect(&self, home: &Path) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_plumb"))
            .args(["doctor", ".", "--json"])
            .current_dir(self.0)
            .env("PLUMB_HOME", home)
            .output()
            .expect("doctor")
    }

    fn git(&self, args: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(self.0)
            .args(args)
            .output()
            .expect("git");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout)
            .expect("git text")
            .trim()
            .to_string()
    }

    fn surface(&self, home: &Path) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_plumb"))
            .args(["ship", "surface"])
            .env("PLUMB_HOME", home)
            .env("PLUMB_RELEASE_ROOT", self.0)
            .output()
            .expect("surface")
    }
}

#[test]
fn boundary() {
    let repo = super::precommit::Repo::new();
    let output = repo.plumb("src");
    assert!(!output.status.success());
    assert!(output.stderr.is_empty());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("report");
    assert_eq!(report["schema"], "plumb.precommit/v1");
    assert_eq!(report["base"], repo.base);
    assert_eq!(report["head"], repo.head);
    assert_eq!(
        report["changed"],
        serde_json::json!(["README.md", "src/lib.rs"])
    );
    assert_eq!(report["outside"], serde_json::json!(["README.md"]));
    assert_eq!(report["ok"], false);
}
