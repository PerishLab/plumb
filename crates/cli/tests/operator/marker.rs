use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;

#[allow(dead_code)]
pub fn line(root: &Path, bare: &Path, version: &str) {
    use std::os::unix::fs::PermissionsExt as _;
    let home = tempfile::tempdir().unwrap().keep();
    let manifest = format!(
        "[[layout.file]]\nname=['Cargo.toml']\nrule=['rule://seat/compiler']\n{}",
        std::fs::read_to_string(root.join("plumb.toml")).unwrap()
    );
    std::fs::write(root.join("plumb.toml"), &manifest).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\n[workspace.package]\nversion='1.2.0'\n",
    )
    .unwrap();
    let ssh = home.join("ssh");
    std::fs::write(
        &ssh,
        format!("#!/bin/sh\nexec git-upload-pack '{}'\n", bare.display()),
    )
    .unwrap();
    std::fs::set_permissions(&ssh, std::fs::Permissions::from_mode(0o755)).unwrap();
    git(root, &["config", "core.sshCommand", ssh.to_str().unwrap()]);
    git(root, &["config", "ssh.variant", "simple"]);
    git(
        root,
        &[
            "remote",
            "set-url",
            "--push",
            "origin",
            bare.to_str().unwrap(),
        ],
    );
    prepare(root, &home, &manifest, version);
    git(root, &["config", "plumb.test-home", home.to_str().unwrap()]);
}

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

pub fn configuration(home: &Path, manifest: &str) -> Value {
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
    let workflow = workflow(product);
    crate::support::stock(
        &source.path().join("configurations"),
        &[
            ("rules/products.toml", &catalog),
            (&address, &profile),
            ("rules/workflow.toml", &workflow),
            (
                "rules/seat.toml",
                "[member]\n[[member.entry]]\nname='compiler'\n[[member.entry.probe]]\nargv=['cargo','--version']\nstdout='fixture cargo'\n",
            ),
        ],
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

#[allow(dead_code)]
pub fn workflow(product: &str) -> String {
    let mut workflow: toml::Value =
        toml::from_str(&crate::support::policy("rules/workflow.toml")).unwrap();
    let inputs: toml::Value = toml::from_str(
        "[binary]\nsource='product'\npaths=['Cargo.toml','Cargo.lock','crates']\n['ship/cargo']\nsource='product'\npaths=['Cargo.toml','Cargo.lock','crates']\n['ship/chart']\nsource='product'\npaths=['charts']\n['ship/oci']\nsource='product'\npaths=['Containerfile']\n",
    ).unwrap();
    let table = workflow.as_table_mut().unwrap();
    table
        .entry("inputs")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()))
        .as_table_mut()
        .unwrap()
        .insert(product.into(), inputs);
    toml::to_string(&workflow).unwrap()
}

pub fn proof(tree: &str, repository: &str) -> String {
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
        bootstrap: None,
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
#[allow(dead_code)]
pub fn controller() -> &'static Path {
    static HELD: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    HELD.get_or_init(|| {
        let source = env!("CARGO_BIN_EXE_plumb");
        let original = std::fs::read(source).unwrap();
        let binding = plumb::identity::Binding {
            product: "plumb".into(),
            marker: plumb::version!("PLUMB").into(),
            digest: "2".repeat(64),
            commit: "3".repeat(40),
            workload: "4".repeat(64),
        };
        let bound = plumb::identity::bind(&original, &binding).unwrap();
        let root = tempfile::Builder::new()
            .prefix("plumb-test-controller-")
            .tempdir()
            .unwrap()
            .keep();
        let path = root.join("plumb");
        std::fs::write(&path, bound).unwrap();
        std::fs::set_permissions(&path, std::fs::metadata(source).unwrap().permissions()).unwrap();
        if cfg!(target_os = "macos") {
            assert!(
                std::process::Command::new("codesign")
                    .args(["--force", "--sign", "-"])
                    .arg(&path)
                    .status()
                    .unwrap()
                    .success()
            );
        }
        let actual = std::fs::read(&path).unwrap();
        assert_eq!(plumb::identity::inspect(&actual).unwrap().1, Some(binding));
        assert_eq!(std::fs::read(source).unwrap(), original);
        path
    })
}
