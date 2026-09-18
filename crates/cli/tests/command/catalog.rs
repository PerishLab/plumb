use std::process::Command;

use super::depot::support;

fn probe(home: &std::path::Path) -> String {
    let root = tempfile::tempdir().unwrap();
    for args in [
        vec!["init", "-q"],
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
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["authority", "ship"])
        .arg(root.path())
        .env("PLUMB_HOME", home)
        .env("PLUMB_AUTHORITY_ACCOUNT", "")
        .env("PLUMB_AUTHORITY_TOKEN", "")
        .output()
        .unwrap();
    assert!(!output.status.success());
    String::from_utf8(output.stderr).unwrap()
}

#[test]
fn inline() {
    let catalog = "schema = 'plumb.products/v1'\n[[product]]\nidentity = 'git.perish.top/PerishFire/concord'\nname = 'concord'\nauthority = 'https://releases.concord.perish.uk'\nderivatives = ['skill']\n";
    let depot = support::depot(&[
        ("rules/products.toml", catalog),
        ("rules/migrations.toml", "schema='plumb.migrations/v1'\n"),
    ]);
    assert!(probe(depot.path()).contains("missing PLUMB_AUTHORITY_ACCOUNT"));
}

#[test]
fn profile() {
    let profile = "schema = 'plumb.product-profile/v1'\n[product]\nname = 'concord'\nauthority = 'https://releases.concord.perish.uk'\nderivatives = ['skill']\n[governance]\nmanifest = ''\nectropy = ''\n";
    let digest = plumb::depot::sha(profile.as_bytes());
    let path = format!("profiles/{digest}.toml");
    let catalog = format!(
        "schema = 'plumb.products/v2'\n[[product]]\nidentity = 'git.perish.top/PerishFire/concord'\nprofile = '{digest}'\n"
    );
    let depot = support::depot(&[("rules/products.toml", &catalog), (&path, profile)]);
    assert!(probe(depot.path()).contains("missing PLUMB_AUTHORITY_ACCOUNT"));
    let drift = support::depot(&[("rules/products.toml", &catalog), (&path, "drift")]);
    assert!(probe(drift.path()).contains("product profile digest drift"));
}

#[test]
fn absent() {
    let empty = tempfile::tempdir().unwrap();
    assert!(probe(empty.path()).contains("verified Depot configuration"));
    let depot = support::depot(&[("rules/products.toml", "schema = 'plumb.products/v1'\n")]);
    assert!(probe(depot.path()).contains("names no products"));
}
