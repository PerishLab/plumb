use sha2::{Digest, Sha256};
use std::process::Command;

#[path = "exact.rs"]
mod exact;

const SEALED: [&str; 4] = ["cargo", "chart", "npm", "oci"];
const PRODUCT: &str = "[release]\nproduct = \"family\"\nauthority = \"https://example.invalid\"\nbinaries = [\"family\"]\ntargets = [\"x86_64-unknown-linux-gnu\"]\n\n[release.cargo]\nregistry = \"perish\"\npackages = [\"family-macro\", \"family-core\"]\n";
const ATTACHMENT: &str = "[release]\nproduct = \"family\"\nauthority = \"https://example.invalid\"\n\n[release.cargo]\nregistry = \"perish\"\npackages = [\"family-macro\", \"family-core\"]\n";

#[test]
fn sealed() {
    let root = tempfile::tempdir().expect("capsule fixture");
    let path = root.path();
    std::fs::write(path.join("plumb.toml"), PRODUCT).expect("attachment");
    let ship = |adaptor: &str, version: &str| {
        Command::new(env!("CARGO_BIN_EXE_plumb"))
            .args(["ship", adaptor, "publish"])
            .env("PLUMB_RELEASE_ROOT", path)
            .env("PLUMB_RELEASE_OUTPUT", ".plumb-release")
            .env("PLUMB_RELEASE_VERSION", version)
            .env("PLUMB_RELEASE_REGISTRY_TOKEN", "secret")
            .output()
            .expect("plumb should run")
    };
    for adaptor in SEALED {
        let bare = ship(adaptor, "v0.10.2-beta.1");
        let missing = String::from_utf8_lossy(&bare.stderr).to_string();
        assert!(
            !bare.status.success() && missing.contains("capsule.json"),
            "{adaptor}: {missing}"
        );
    }

    let out = path.join(".plumb-release");
    std::fs::create_dir_all(&out).expect("release output");
    let body = b"{}";
    let record = out.join("seal.json");
    std::fs::write(&record, body).expect("seal");
    let served = format!("file://{}", record.display());
    let digest = format!("{:x}", Sha256::digest(body));
    let capsule = format!(
        concat!(
            r#"{{"schema":1,"product":"family","channel":"beta","#,
            r#""releaseVersion":"v0.10.2-beta.1","authority":"https://example.invalid","#,
            r#""objects":[],"seal":{{"source":"seal.json","key":"v1/seal.json","#,
            r#""remote":{{"name":"seal.json","mime":"application/json","sha256":"{}","#,
            r#""size":{},"url":"{}"}}}},"#,
            r#""roots":[],"pointer":null}}"#
        ),
        digest,
        body.len(),
        served
    );
    std::fs::write(out.join("capsule.json"), &capsule).expect("capsule");

    for adaptor in SEALED {
        let drift = ship(adaptor, "v0.10.2-beta.2");
        let refused = String::from_utf8_lossy(&drift.stderr).to_string();
        assert!(
            !drift.status.success()
                && refused.contains(
                    "capsule seals v0.10.2-beta.1 while the projection carries v0.10.2-beta.2"
                ),
            "{adaptor}: {refused}"
        );
    }

    for (adaptor, medium) in [("chart", "chart"), ("npm", "module"), ("oci", "image")] {
        let absent = ship(adaptor, "v0.10.2-beta.1");
        let said = String::from_utf8_lossy(&absent.stdout).to_string();
        assert!(
            absent.status.success() && said.contains(&format!("has no {medium} attachment")),
            "{adaptor}: {said}"
        );
    }

    std::fs::create_dir_all(path.join("packages/family")).expect("module seat");
    std::fs::write(
        path.join("plumb.toml"),
        format!(
            "{PRODUCT}\n[release.npm]\nregistry = \"https://example.invalid/npm/\"\npackages = [\"@family/family\"]\n"
        ),
    )
    .expect("attachment");
    let malformed = ship("npm", "v0.10.2-beta.1");
    let refused = String::from_utf8_lossy(&malformed.stderr).to_string();
    assert!(
        !malformed.status.success() && refused.contains("must be a Cargo Bearer credential"),
        "{refused}"
    );

    let elsewhere = out.join("elsewhere.json");
    std::fs::write(&elsewhere, b"{\"other\":true}").expect("served");
    std::fs::write(
        out.join("capsule.json"),
        capsule.replace(&served, &format!("file://{}", elsewhere.display())),
    )
    .expect("capsule");
    let drifted = ship("npm", "v0.10.2-beta.1");
    let said = String::from_utf8_lossy(&drifted.stderr).to_string();
    assert!(
        !drifted.status.success() && said.contains("public object drift"),
        "{said}"
    );
}

#[test]
fn unsealed() {
    let root = tempfile::tempdir().expect("attachment fixture");
    let path = root.path();
    std::fs::write(path.join("plumb.toml"), ATTACHMENT).expect("attachment");
    let out = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["ship", "cargo", "publish"])
        .env("PLUMB_RELEASE_ROOT", path)
        .env_remove("PLUMB_RELEASE_OUTPUT")
        .env_remove("PLUMB_RELEASE_CAPSULE")
        .env("PLUMB_RELEASE_VERSION", "v0.10.2-beta.1")
        .env("PLUMB_RELEASE_REGISTRY_TOKEN", "Bearer secret")
        .output()
        .expect("plumb should run");
    let reached = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(!reached.contains("PLUMB_RELEASE_OUTPUT"), "{reached}");
    assert!(!reached.contains("capsule"), "{reached}");
    assert!(
        reached.contains("cannot read") && reached.contains("Cargo.toml"),
        "{reached}"
    );
}

#[test]
fn settled() {
    let temp = tempfile::tempdir().expect("temp root");
    let root = temp.path();
    std::fs::create_dir_all(root.join("packages/held")).expect("package root");
    std::fs::write(
        root.join("plumb.toml"),
        "[release.npm]\nregistry = \"https://registry.invalid\"\npackages = [\"held\"]\n",
    )
    .expect("manifest");
    std::fs::write(
        root.join("packages/held/package.json"),
        "{\"name\":\"held\"}\n",
    )
    .expect("package");
    for args in [
        vec!["init", "-q"],
        vec!["config", "user.name", "Fixture"],
        vec!["config", "user.email", "fixture@example.test"],
        vec!["add", "-A"],
        vec!["commit", "-qm", "seed"],
    ] {
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .status()
            .expect("git");
        assert!(status.success());
    }
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["ship", "npm", "pack"])
        .env("PLUMB_RELEASE_ROOT", root)
        .env("PLUMB_RELEASE_VERSION", "v1.2.0-beta.1")
        .output()
        .expect("plumb should run");
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        !text.contains("unchanged since"),
        "a product with no authority holds no baseline, so nothing is settled: {text}"
    );
    let exact = |package: &str, reuse: &str| {
        std::process::Command::new(env!("CARGO_BIN_EXE_plumb"))
            .args([
                "ship",
                "npm",
                "exact",
                "--package",
                package,
                "--reuse",
                reuse,
            ])
            .env("PLUMB_RELEASE_ROOT", root)
            .env("PLUMB_RELEASE_VERSION", "v1.2.0-beta.1")
            .env("PLUMB_RELEASE_REGISTRY_TOKEN", "Bearer secret")
            .output()
            .expect("plumb should run")
    };
    let unknown = exact("other", r#"{"type":"none","source":""}"#);
    assert!(
        !unknown.status.success()
            && String::from_utf8_lossy(&unknown.stderr)
                .contains("other is not a declared module attachment")
    );
    let published = exact(
        "held",
        r#"{"type":"url","source":"https://registry.invalid/held"}"#,
    );
    assert!(
        !published.status.success()
            && String::from_utf8_lossy(&published.stderr)
                .contains("held publication URL must skip the module action")
    );
}
