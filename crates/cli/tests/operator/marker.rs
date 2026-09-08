use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;

#[allow(dead_code)]
pub fn prepare(root: &Path, home: &Path, manifest: &str, version: &str) -> Value {
    let binding = configuration(home, manifest);
    let product = binding["product"].as_str().unwrap();
    git(root, &["init", "-q"]);
    git(root, &["config", "user.name", "Fixture"]);
    git(root, &["config", "user.email", "fixture@example.test"]);
    std::fs::write(
        root.join(".gitignore"),
        "target/\nhome/\nbin/\ntools/\nartifacts/\nbinary/\n*.tgz\n*.tar\napps/*/dist/\n",
    )
    .unwrap();
    let remote = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["remote", "get-url", "origin"])
        .output()
        .unwrap();
    let verb = if remote.status.success() {
        "set-url"
    } else {
        "add"
    };
    git(
        root,
        &[
            "remote",
            verb,
            "origin",
            &format!("ssh://git@git.perish.top/PerishFire/{product}.git"),
        ],
    );
    let base = version.split('-').next().unwrap();
    let datum = root.join(plumb::datum::leaf(base));
    std::fs::create_dir_all(datum.parent().unwrap()).unwrap();
    std::fs::write(datum, format!("schema = 1\nversion = {base:?}\n")).unwrap();
    git(root, &["add", "."]);
    let tree = git(root, &["write-tree"]);
    let proof = proof(&tree, &format!("PerishFire/{product}"));
    git(
        root,
        &[
            "commit",
            "--allow-empty",
            "-qm",
            &format!("fixture\n\nPlumb-Guard-Proof: {proof}"),
        ],
    );
    git(
        root,
        &[
            "update-ref",
            &format!("refs/remotes/origin/release/{base}"),
            "HEAD",
        ],
    );
    let annotation = json!({
        "schema":"plumb.release-marker/v3", "product":product, "marker":version,
        "configuration":{"channel":"stable","version":plumb::version!("PLUMB").to_string(),"generation":binding["configuration"]},
        "profile":binding["profile"],
    });
    git(root, &["tag", "-a", version, "-m", &annotation.to_string()]);
    binding
}

fn configuration(home: &Path, manifest: &str) -> Value {
    let source = tempfile::tempdir().unwrap();
    let document: toml::Value = toml::from_str(manifest).unwrap();
    let product = document["release"]["product"].as_str().unwrap();
    let authority = document["release"]["authority"].as_str().unwrap();
    let profile = format!(
        "schema='plumb.product-profile/v1'\n[product]\nname={product:?}\nauthority={authority:?}\nderivatives=['skill']\n[governance]\nmanifest={manifest:?}\nectropy='[comment]'\n"
    );
    let digest = plumb::depot::sha(profile.as_bytes());
    let catalog = format!(
        "schema='plumb.products/v2'\n[[product]]\nidentity='git.perish.top/PerishFire/{product}'\nprofile='{digest}'\n"
    );
    let address = format!("profiles/{digest}.toml");
    crate::support::stock(
        &source.path().join("configurations"),
        &[("rules/products.toml", &catalog), (&address, &profile)],
    );
    let version = plumb::version!("PLUMB").to_string();
    let bundle = plumb::depot::v3::Bundle::read(
        &source.path().join("configurations/29990101T000000Z"),
        plumb::depot::v3::Identity {
            product: "plumb".into(),
            channel: "stable".into(),
            version: version.clone(),
            marker: plumb::depot::v3::Marker {
                name: version,
                sha256: "a".repeat(64),
            },
            kind: plumb::depot::v3::Kind::Configuration,
        },
    )
    .unwrap();
    let pointer = plumb::depot::v3::Pointer::new(
        &bundle.manifest,
        plumb::depot::v3::Publication {
            source: "https://depot.test",
            prior: None,
            created: "2026-09-08T00:00:00Z".into(),
        },
    )
    .unwrap();
    plumb::depot::v3::install(&home.join("configurations"), &pointer, &bundle).unwrap();
    json!({"configuration":pointer.generation,"profile":digest,"product":product})
}

fn proof(tree: &str, repository: &str) -> String {
    #[derive(serde::Serialize)]
    struct Claim<'a> {
        schema: &'a str,
        repository: &'a str,
        tree: &'a str,
        plumb: &'a str,
        depot: &'a str,
        platform: &'a str,
        actions: &'a [plumb::guard::Action],
    }
    let version = plumb::version!("PLUMB").to_string();
    let depot = "2".repeat(64);
    let platform = plumb::config::platform();
    let actions = vec![plumb::guard::Action {
        name: "guard/test".into(),
        input: "0".repeat(64),
        world: "1".repeat(64),
    }];
    let claim = Claim {
        schema: plumb::guard::SCHEMA,
        repository,
        tree,
        plumb: &version,
        depot: &depot,
        platform: &platform,
        actions: &actions,
    };
    let digest = plumb::depot::sha(&serde_json::to_vec(&claim).unwrap());
    plumb::guard::Descriptor {
        schema: plumb::guard::SCHEMA.into(),
        repository: repository.into(),
        tree: tree.into(),
        plumb: version,
        depot,
        platform,
        actions,
        digest,
    }
    .encode()
    .unwrap()
}

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}
