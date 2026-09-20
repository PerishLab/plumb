use super::super::support;
use super::Repo;
use super::WORKFLOW;
use std::process::Command;

#[test]
#[cfg(unix)]
fn governed() {
    let fixture = tempfile::tempdir().expect("fixture");
    let seat = fixture.path().join("probe");
    std::fs::create_dir(&seat).expect("repository");
    let root = seat.as_path();
    Repo::git(root, &["init", "-q"]);
    Repo::git(root, &["config", "user.name", "Plumb Test"]);
    Repo::git(root, &["config", "user.email", "plumb@example.invalid"]);
    Repo::git(
        root,
        &[
            "remote",
            "add",
            "origin",
            "ssh://git@git.perish.top/PerishFire/probe.git",
        ],
    );
    std::fs::create_dir(root.join("src")).expect("source");
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"probe\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("manifest");
    std::fs::write(root.join(".gitignore"), "target/\n").expect("ignore");
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn answer() -> u8 {\n    42\n}\n",
    )
    .expect("source");
    let lock = Command::new("cargo")
        .args(["generate-lockfile", "--offline"])
        .current_dir(root)
        .output()
        .expect("lock");
    assert!(
        lock.status.success(),
        "{}",
        String::from_utf8_lossy(&lock.stderr)
    );
    Repo::git(
        root,
        &[
            "add",
            ".gitignore",
            "Cargo.toml",
            "Cargo.lock",
            "src/lib.rs",
        ],
    );
    let profile = super::super::world::profile(
        "probe",
        "[layout]\n[[layout.seat]]\npath = \"src\"\n[[layout.file]]\nname = [\".gitignore\", \"Cargo.toml\", \"Cargo.lock\"]\n",
        "[comment]\nallow = false\n[limit]\nblock = 4\nfanout = 10\nfile = 300\nmarkup = 8\nparam = 4\npath = 3\n[word]\nsingle = true\n",
    );
    let digest = plumb::depot::sha(profile.as_bytes());
    let catalog = format!(
        "schema = \"plumb.products/v2\"\n\n[[product]]\nidentity = \"git.perish.top/PerishFire/probe\"\nprofile = \"{digest}\"\n"
    );
    let path = format!("profiles/{digest}.toml");
    let home = support::home(&[
        ("rules/products.toml", &catalog),
        (&path, &profile),
        ("rules/workflow.toml", WORKFLOW),
    ]);
    super::super::world::hooks(root);
    let binary = std::path::Path::new(env!("CARGO_BIN_EXE_plumb"));
    let tools = fixture.path().join("tools");
    std::fs::create_dir(&tools).expect("tool root");
    std::fs::write(
        tools.join("ectropy"),
        "#!/bin/sh\n[ \"$1\" = --version ] && echo \"ectropy 0.0.0\"\nexit 0\n",
    )
    .expect("ectropy stub");
    std::fs::set_permissions(
        tools.join("ectropy"),
        std::os::unix::fs::PermissionsExt::from_mode(0o755),
    )
    .expect("ectropy stub mode");
    let path = format!(
        "{}:{}:{}",
        tools.display(),
        binary.parent().expect("Plumb binary directory").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let run = || {
        support::plumb()
            .args(["guard", "."])
            .current_dir(root)
            .env_remove("PLUMB_HOME")
            .env_remove("PLUMB_GUARD_CONFIGURATION")
            .env_remove("PLUMB_GUARD_DEPOT")
            .env_remove("PLUMB_GUARD_VIEW")
            .env("HOME", home.path())
            .env("PATH", &path)
            .output()
            .expect("guard")
    };
    let output = run();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let held = String::from_utf8_lossy(&output.stderr);
    assert!(held.contains("guard guard/plumb"), "{held}");
    assert!(held.contains("guard guard/ectropy"), "{held}");
    assert!(!held.contains("must not carry plumb.toml or ectropy.toml"));
    assert!(!root.join("plumb.toml").exists());
    assert!(!root.join("ectropy.toml").exists());
    std::fs::write(root.join("ectropy.toml"), "").expect("second expression");
    Repo::git(root, &["add", "ectropy.toml"]);
    let refusal = run();
    assert!(!refusal.status.success());
    assert!(
        String::from_utf8_lossy(&refusal.stderr)
            .contains("must not carry plumb.toml or ectropy.toml")
    );
}
