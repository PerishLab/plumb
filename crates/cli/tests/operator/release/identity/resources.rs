use std::path::Path;
use std::process::Command;

#[path = "../../../../src/command/ship/transport/sources.rs"]
mod sources;

#[test]
fn resources() {
    let seat = tempfile::tempdir().expect("repository");
    let root = seat.path();
    git(root, &["init", "-q"]);
    for (path, body) in [
        ("Cargo.toml", "[workspace]\nmembers = [\"crates/cli\"]\n"),
        (
            "crates/cli/Cargo.toml",
            "[package]\nname = \"probe\"\nversion = \"1.0.0\"\n",
        ),
        ("crates/cli/src/main.rs", "fn main() {}"),
        ("crates/cli/cookbook/index.txt", "first"),
        ("crates/cli/help/usage.txt", "help"),
        ("crates/cli/assets/icon.txt", "icon"),
        ("crates/cli/tests/probe.rs", "test"),
        ("AGENTS.md", "operations"),
    ] {
        write(root, path, body);
    }
    git(root, &["add", "."]);
    git(
        root,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.test",
            "commit",
            "-qm",
            "source",
        ],
    );
    let paths = sources::read(root).expect("production inputs");
    assert!(paths.contains("crates/cli/cookbook"));
    assert!(paths.contains("crates/cli/help"));
    assert!(paths.contains("crates/cli/assets"));
    let digest = || {
        fingerprint(
            root,
            serde_json::json!({"paths":sources::read(root).unwrap()}),
        )
    };
    let first = digest();
    write(root, "crates/cli/tests/probe.rs", "changed test");
    write(root, "AGENTS.md", "changed operations");
    git(root, &["add", "."]);
    assert_eq!(digest(), first);
    write(root, "crates/cli/cookbook/index.txt", "second");
    git(root, &["add", "."]);
    assert_ne!(digest(), first);
}

pub(crate) fn fingerprint(root: &Path, recipe: serde_json::Value) -> String {
    git(
        root,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.test",
            "commit",
            "--allow-empty",
            "-qm",
            "snapshot",
        ],
    );
    let scripts = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.forgejo/scripts");
    let mut command = Command::new("python3");
    command.current_dir(scripts).args(["-c", "import json,sys; from lib.source import Source; print(Source(sys.argv[1]).fingerprint(json.loads(sys.argv[2])))"])
        .arg(root).arg(recipe.to_string());
    let output = command.output().expect("input plan");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

fn write(root: &Path, path: &str, body: &str) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    std::fs::write(path, body).expect("file");
}

fn git(root: &Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .status()
            .expect("git")
            .success()
    );
}

pub(super) fn candidate(fixture: &super::super::world::Fixture<'_>, source: &Path) {
    use std::os::unix::fs::PermissionsExt as _;
    let manifest = std::fs::read_to_string(fixture.root.join("plumb.toml")).unwrap();
    let manifest =
        format!("{manifest}\n[[layout.file]]\nname=['AGENTS.md']\nrule=['rule://seat/affirmed']\n");
    let profile = format!(
        "schema='plumb.product-profile/v1'\n[product]\nname='probe'\nauthority='https://releases.test'\ndepot='https://depot.test'\nderivatives=['configuration']\n[governance]\nmanifest={manifest:?}\nectropy='[comment]'\n"
    );
    let digest = plumb::depot::sha(profile.as_bytes());
    let catalog = "schema='plumb.products/v1'\n[[product]]\nidentity='git.perish.top/PerishLab/probe'\nname='probe'\nauthority='https://releases.test'\ndepot='https://depot.test'\nderivatives=['configuration']\n";
    let migration = format!(
        "schema='plumb.migrations/v1'\n[[product]]\nidentity='git.perish.top/PerishLab/probe'\nprofile='{digest}'\nsource='repository'\n"
    );
    write(source, "rules/products.toml", catalog);
    write(source, "rules/migrations.toml", &migration);
    write(
        source,
        "rules/seat.toml",
        "[[member.entry]]\nname='affirmed'\naffirms=['declaration','seat','lane']\n",
    );
    write(source, &format!("profiles/{digest}.toml"), &profile);
    super::super::support::stock(
        &fixture.root.join("configurations"),
        &[
            ("rules/products.toml", catalog),
            ("rules/migrations.toml", "schema='plumb.migrations/v1'\n"),
        ],
    );
    std::fs::write(fixture.root.join("ectropy.toml"), "caller policy").unwrap();
    let real = Command::new("sh")
        .args(["-c", "command -v git"])
        .output()
        .unwrap();
    assert!(real.status.success());
    let real = String::from_utf8(real.stdout).unwrap().trim().to_string();
    let shim = fixture.tools.join("git");
    std::fs::write(&shim, format!(
        "#!/bin/sh\ncase \"$*\" in\n*\"remote get-url origin\") echo https://git.perish.top/PerishLab/probe.git ;;\n*) exec {real:?} \"$@\" ;;\nesac\n"
    )).unwrap();
    std::fs::set_permissions(shim, std::fs::Permissions::from_mode(0o755)).unwrap();
    let refused = fixture
        .command()
        .current_dir(fixture.root)
        .args([
            "depot",
            "configuration",
            "--marker",
            "v1.2.0-beta.2",
            "--from",
        ])
        .arg(source)
        .arg("--dry-run")
        .output()
        .unwrap();
    assert!(
        !refused.status.success(),
        "candidate must not automatically affirm"
    );
    let confirmed = fixture
        .command()
        .current_dir(fixture.root)
        .args(["affirm", "--configuration"])
        .arg(source)
        .arg("--write")
        .output()
        .unwrap();
    assert!(
        confirmed.status.success(),
        "{}",
        String::from_utf8_lossy(&confirmed.stderr)
    );
}

pub(super) fn unchanged(fixture: &super::super::world::Fixture<'_>, source: &Path, commit: &str) {
    assert_eq!(
        std::fs::read_to_string(fixture.root.join(".plumb/affirmed.toml")).unwrap(),
        "# frozen confirmation\n"
    );
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("ectropy.toml")).unwrap(),
        "caller policy"
    );
    let actual = Command::new("git")
        .arg("-C")
        .arg(fixture.root)
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    assert!(actual.status.success());
    assert_eq!(String::from_utf8_lossy(&actual.stdout).trim(), commit);
    let catalog = std::fs::read_to_string(source.join("rules/products.toml")).unwrap();
    let migration = std::fs::read_to_string(source.join("rules/migrations.toml")).unwrap();
    let path = std::fs::read_dir(source.join("profiles"))
        .unwrap()
        .filter_map(Result::ok)
        .find(|entry| entry.path().is_file())
        .unwrap()
        .path();
    let profile = std::fs::read_to_string(&path).unwrap();
    let address = format!("profiles/{}", path.file_name().unwrap().to_str().unwrap());
    super::super::support::stock(
        &fixture.root.join("configurations"),
        &[
            ("rules/products.toml", &catalog),
            ("rules/migrations.toml", &migration),
            (&address, &profile),
        ],
    );
    let output = fixture
        .command()
        .current_dir(fixture.root)
        .args(["doctor", ".", "--json"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.contains("ectropy.toml differs from its exact Depot profile"),
        "{text}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::write(&path, format!("{profile}\n")).unwrap();
    let refused = fixture
        .command()
        .current_dir(fixture.root)
        .args([
            "depot",
            "configuration",
            "--marker",
            "v1.2.0-beta.3",
            "--from",
            source.to_str().unwrap(),
            "--dry-run",
        ])
        .output()
        .unwrap();
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("product profile digest drift"));
}
