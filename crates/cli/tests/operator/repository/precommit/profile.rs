use std::path::Path;
use std::process::{Command, Output};

fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    for args in [
        vec!["init", "-q"],
        vec!["config", "user.name", "Plumb Test"],
        vec!["config", "user.email", "plumb@example.invalid"],
        vec![
            "remote",
            "add",
            "origin",
            "ssh://git@git.perish.top/PerishLab/plumb.git",
        ],
    ] {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(root.path())
                .status()
                .unwrap()
                .success()
        );
    }
    std::fs::write(
        root.path().join("Cargo.toml"),
        format!(
            "[workspace.package]\nversion = {:?}\n",
            env!("CARGO_PKG_VERSION")
        ),
    )
    .unwrap();
    std::fs::write(
        root.path().join("plumb.toml"),
        "[release]\nproduct='probe'\nauthority='https://releases.probe.perish.uk'\nbinaries=['probe']\ntargets=['x86_64-unknown-linux-gnu']\n",
    )
    .unwrap();
    assert!(
        Command::new("git")
            .args(["add", "."])
            .current_dir(root.path())
            .status()
            .unwrap()
            .success()
    );
    root
}

fn command(root: &Path, verb: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_plumb"));
    command
        .args([verb, ".", "--json"])
        .current_dir(root)
        .env_remove("PLUMB_HOME")
        .env_remove("PLUMB_GUARD_CONFIGURATION")
        .env_remove("PLUMB_GUARD_DEPOT")
        .env_remove("PLUMB_GUARD_VIEW");
    command
}

fn refused(output: Output, expected: &str) {
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!output.status.success(), "{text}");
    assert!(text.contains(expected), "{text}");
}

#[test]
fn absent() {
    let root = fixture();
    let catalog = "schema='plumb.products/v2'\nproduct=[]\n";
    let home = super::support::depot(&[("rules/products.toml", catalog)]);
    refused(
        command(root.path(), "guard")
            .env("PLUMB_HOME", home.path())
            .output()
            .unwrap(),
        "is absent from the Plumb depot",
    );
    let seat = super::support::guard(
        &[("rules/products.toml", catalog)],
        &format!("v{}", env!("CARGO_PKG_VERSION")),
    );
    refused(
        command(root.path(), "doctor")
            .env("PLUMB_GUARD_CONFIGURATION", seat.path())
            .output()
            .unwrap(),
        "is absent from the Plumb depot",
    );
}

#[test]
fn bound() {
    let root = fixture();
    let profile = super::super::world::profile("plumb", "[layout]\n", "[comment]\nallow=false\n");
    let digest = plumb::depot::sha(profile.as_bytes());
    let path = format!("profiles/{digest}.toml");
    let catalog = format!(
        "schema='plumb.products/v2'\n[[product]]\nidentity='git.perish.top/PerishLab/plumb'\nprofile='{digest}'\n"
    );
    let overrides = [
        ("rules/products.toml", catalog.as_str()),
        (path.as_str(), profile.as_str()),
    ];
    let home = super::support::depot(&overrides);
    refused(
        command(root.path(), "guard")
            .env("PLUMB_HOME", home.path())
            .output()
            .unwrap(),
        "must not carry plumb.toml or ectropy.toml",
    );
    let seat = super::support::guard(&overrides, &format!("v{}", env!("CARGO_PKG_VERSION")));
    for projected in [false, true] {
        let mut doctor = command(root.path(), "doctor");
        doctor.env("PLUMB_GUARD_CONFIGURATION", seat.path());
        if projected {
            doctor.env("PLUMB_GUARD_VIEW", "true");
        }
        let output = doctor.output().unwrap();
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["profile"], digest);
        refused(
            output,
            if projected {
                "differs from its exact Depot profile"
            } else {
                "must not carry plumb.toml or ectropy.toml"
            },
        );
    }
}
