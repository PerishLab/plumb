use std::process::Command;

fn surface(root: &std::path::Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["release", "surface"])
        .env("PLUMB_RELEASE_ROOT", root)
        .output()
        .expect("plumb should run");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn declared() {
    let root = std::env::temp_dir().join("plumb-release-surface");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("skills/foo")).expect("fixture");

    std::fs::write(
        root.join("plumb.toml"),
        "[release.cargo]\nregistry = \"perish\"\npackages = [\"foo\"]\n",
    )
    .expect("manifest");
    let cargo: serde_json::Value = serde_json::from_str(&surface(&root)).expect("cargo surface");
    assert_eq!(
        cargo["project"]["include"][0]["schema"],
        "plumb.ship-request/v1"
    );
    assert_eq!(cargo["project"]["include"][0]["operation"]["type"], "cargo");

    std::fs::write(
        root.join("plumb.toml"),
        "[release]\nproduct = \"foo\"\nauthority = \"https://example.invalid\"\nbinaries = [\"foo\"]\ntargets = [\"x86_64-unknown-linux-gnu\"]\nskill = true\n[release.oci]\nregistry = \"example.invalid\"\nimage = \"owner/foo\"\naccount = \"Example\"\n",
    )
    .expect("manifest");
    let oci: serde_json::Value = serde_json::from_str(&surface(&root)).expect("image surface");
    assert_eq!(oci["project"]["include"][0]["action"], "ship/oci");
    assert_eq!(oci["project"]["include"][0]["operation"]["type"], "oci");

    std::fs::remove_dir_all(&root).expect("fixture should be swept");
}

fn refusal(root: &std::path::Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["release", "surface"])
        .env("PLUMB_RELEASE_ROOT", root)
        .output()
        .expect("plumb should run");
    assert!(!output.status.success());
    String::from_utf8_lossy(&output.stderr).trim().to_string()
}

#[test]
fn worker() {
    let root = std::env::temp_dir().join("plumb-release-cfworker");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("apps")).expect("fixture");
    let manifest = root.join("plumb.toml");

    std::fs::write(
        &manifest,
        "[release.cfworker]\naccount = \"held\"\ndomain = \"probe.example.uk\"\n",
    )
    .expect("manifest");
    let worker: serde_json::Value = serde_json::from_str(&surface(&root)).expect("worker surface");
    assert_eq!(worker["project"]["include"][0]["action"], "ship/cfworker");
    assert_eq!(
        worker["project"]["include"][0]["operation"]["type"],
        "cfworker"
    );

    std::fs::write(
        &manifest,
        "[release.cfworker]\naccount = \"held\"\ndomain = \"probe_one.example.uk\"\n",
    )
    .expect("manifest");
    assert!(
        refusal(&root).contains("RFC 1123"),
        "an underscore is not a hostname label"
    );

    std::fs::write(
        &manifest,
        "[release.cfworker]\naccount = \"held\"\ndomain = \"probe.example.uk\"\npreview = \"held.example.uk\"\n",
    )
    .expect("manifest");
    assert!(
        refusal(&root).contains("unknown field"),
        "a preview root is derived, never declared"
    );

    std::fs::remove_dir_all(&root).expect("fixture should be swept");
}

#[test]
fn depends() {
    let root = std::env::temp_dir().join("plumb-release-depends");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("packages/held")).expect("fixture");
    std::fs::create_dir_all(root.join("packages/bone")).expect("fixture");
    let manifest = root.join("plumb.toml");

    std::fs::write(
        &manifest,
        "[release.npm]\nregistry = \"https://example.invalid\"\npackages = [\"held\"]\n[release.depends]\n\"npm/held\" = [\"packages/bone\"]\n",
    )
    .expect("manifest");
    assert!(
        surface(&root).contains("npm"),
        "a declared extra root does not change the surface"
    );

    std::fs::write(
        &manifest,
        "[release.npm]\nregistry = \"https://example.invalid\"\npackages = [\"held\"]\n[release.depends]\n\"npm/absent\" = [\"packages/bone\"]\n",
    )
    .expect("manifest");
    assert!(
        surface(&root).contains("npm"),
        "the surface reads before objects resolve"
    );

    std::fs::remove_dir_all(&root).expect("fixture should be swept");
}

#[test]
fn prepared() {
    let root = std::env::temp_dir().join("plumb-release-prepared");
    let _ = std::fs::remove_dir_all(&root);
    for seat in ["apps/web", "charts/probe", "packages/probe"] {
        std::fs::create_dir_all(root.join(seat)).expect("fixture");
    }
    std::fs::write(
        root.join("plumb.toml"),
        concat!(
            "[release.cargo]\nregistry = \"perish\"\npackages = [\"probe\"]\n",
            "[release.chart]\nregistry = \"example.invalid\"\nchart = \"owner/probe\"\naccount = \"Example\"\n",
            "[release.npm]\nregistry = \"https://example.invalid\"\npackages = [\"probe\"]\n",
            "[release.cfworker]\naccount = \"held\"\ndomain = \"probe.example.uk\"\n",
            "[release.oci]\nregistry = \"example.invalid\"\nimage = \"owner/probe\"\naccount = \"Example\"\n",
        ),
    )
    .expect("manifest");
    let plan: serde_json::Value =
        serde_json::from_str(&surface(&root)).expect("the surface is one plan");
    let rows = plan["project"]["include"]
        .as_array()
        .expect("a plan names the projected media");
    assert_eq!(rows.len(), 5, "{rows:?}");
    let npm = rows
        .iter()
        .find(|row| row["operation"]["type"] == "npm")
        .expect("npm project");
    assert_eq!(npm["action"], "ship/npm.probe");
    assert_eq!(
        npm["projections"],
        serde_json::json!(["packages/probe/package.json#/version"])
    );
    assert_eq!(npm["roots"], serde_json::json!(["packages/probe"]));
    let chart = rows
        .iter()
        .find(|row| row["operation"]["type"] == "chart")
        .expect("chart project");
    assert_eq!(chart["action"], "ship/chart");
    assert_eq!(
        chart["projections"],
        serde_json::json!([
            "charts/probe/Chart.yaml#/version",
            "charts/probe/Chart.yaml#/appVersion"
        ])
    );
    assert_eq!(chart["roots"], serde_json::json!(["charts/probe"]));
    for row in rows {
        assert_eq!(row["schema"], "plumb.ship-request/v1");
        assert!(row["action"].as_str().is_some_and(|held| !held.is_empty()));
        assert!(row["roots"].as_array().is_some_and(|held| !held.is_empty()));
    }
    let held = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["ship", "execute", "--help"])
        .output()
        .expect("plumb should run");
    assert!(held.status.success(), "the common executor must exist");
    std::fs::remove_dir_all(&root).expect("fixture should be swept");
}
