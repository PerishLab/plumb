use super::goods::{self, Consignment, SCHEMA};
use sha2::{Digest, Sha256};
use std::path::Path;

fn write(root: &Path, path: &str, body: &str, mode: u32) {
    use std::os::unix::fs::PermissionsExt as _;
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    std::fs::write(&path, body).expect("write");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).expect("mode");
}

#[test]
fn directory() {
    let fixture = tempfile::tempdir().expect("fixture");
    write(fixture.path(), "zh/INDEX.md", "中文\n", 0o644);
    write(fixture.path(), "en/INDEX.md", "notes\n", 0o644);
    write(fixture.path(), "run.sh", "#!/bin/sh\n", 0o755);
    let held = goods::directory(fixture.path()).expect("goods");
    let named = held
        .iter()
        .map(|good| (good.path.as_str(), good.executable))
        .collect::<Vec<_>>();
    assert_eq!(
        named,
        [
            ("en/INDEX.md", false),
            ("run.sh", true),
            ("zh/INDEX.md", false)
        ]
    );
    assert_eq!(held[0].sha256, format!("{:x}", Sha256::digest(b"notes\n")));
    std::os::unix::fs::symlink("run.sh", fixture.path().join("link")).expect("symlink");
    assert!(
        goods::directory(fixture.path())
            .expect_err("a link refuses")
            .contains("not a plain file")
    );
}

#[test]
fn brief() {
    let fixture = tempfile::tempdir().expect("fixture");
    let root = fixture.path();
    let absent = super::vet(root, "SKILL.md", Some(3072)).expect_err("no brief");
    assert!(absent.contains("carries no SKILL.md"), "{absent}");
    write(root, "SKILL.md", "# demo\n", 0o644);
    super::vet(root, "SKILL.md", Some(3072)).expect("a brief within the cap");
    write(root, "SKILL.md", &"x".repeat(3073), 0o644);
    assert!(
        super::vet(root, "SKILL.md", Some(3072))
            .expect_err("over the cap")
            .contains("caps 3072")
    );
}

#[test]
fn encoded() {
    let objects = [goods::good("en/INDEX.md".into(), b"notes\n", false)];
    let (body, digest) = Consignment {
        schema: SCHEMA,
        repository: "PerishLab/demo",
        marker: "v1.0.0",
        kind: "changelog",
        objects: &objects,
    }
    .encode()
    .expect("encode");
    assert_eq!(digest, format!("{:x}", Sha256::digest(&body)));
    let held: serde_json::Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(held["schema"], "wharf.yard/v1");
    assert_eq!(held["objects"][0]["body"], "bm90ZXMK");
    assert_eq!(held["objects"][0]["executable"], false);
}
