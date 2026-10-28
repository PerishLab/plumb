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
    write(root, "PATHS.md", "# paths\n", 0o644);
    let companion = super::vet(root, "SKILL.md", Some(3072)).expect_err("a companion refuses");
    assert!(companion.contains("entries beside SKILL.md"), "{companion}");
    std::fs::remove_file(root.join("PATHS.md")).expect("remove companion");
    std::fs::create_dir(root.join("references")).expect("references");
    let directory = super::vet(root, "SKILL.md", Some(3072)).expect_err("a directory refuses");
    assert!(directory.contains("entries beside SKILL.md"), "{directory}");
    std::fs::remove_dir(root.join("references")).expect("remove references");
    write(root, "SKILL.md", &"x".repeat(3073), 0o644);
    assert!(
        super::vet(root, "SKILL.md", Some(3072))
            .expect_err("over the cap")
            .contains("caps 3072")
    );
    std::fs::remove_file(root.join("SKILL.md")).expect("remove brief");
    let target = tempfile::NamedTempFile::new().expect("target");
    std::os::unix::fs::symlink(target.path(), root.join("SKILL.md")).expect("brief link");
    let link = super::vet(root, "SKILL.md", Some(3072)).expect_err("a link refuses");
    assert!(link.contains("is not a regular file"), "{link}");
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

#[test]
fn scenes() {
    let fixture = tempfile::tempdir().expect("fixture");
    let root = fixture.path();
    assert!(
        goods::directory(root)
            .expect_err("empty")
            .contains("nothing")
    );
    write(
        root,
        "writer.toml",
        "command = [\"sh\", \"run.sh\"]\n",
        0o644,
    );
    write(root, "run.sh", "exit 91\n", 0o755);
    let options = super::Consign {
        version: "v1.2.3".into(),
        kind: "scene".into(),
        dir: Some(root.into()),
        unwatched: false,
        dry: true,
    };
    let spec = crate::shape::release::Spec::decode(
        root,
        "[identity]\nns = \"probe\"\n[release]\nauthority = \"https://releases.probe.example\"\nbinaries = [\"probe\"]\ntargets = [\"x86_64-unknown-linux-gnu\"]\n",
        "scene fixture",
    )
    .expect("release declaration");
    let (objects, kind) = super::gathered(&options, root, &spec, "v1.2.3")
        .expect("scenes do not require a skill or execute their commands");
    assert_eq!(kind, plumb::depot::v3::Kind::Scene);
    assert_eq!(objects.len(), 2);
    assert_eq!(objects[0].path, "run.sh");
    assert!(objects[0].executable);
    assert_eq!(
        objects[0].sha256,
        format!("{:x}", Sha256::digest(b"exit 91\n"))
    );
    let missing = super::Consign {
        dir: None,
        ..options
    };
    assert!(
        super::gathered(&missing, root, &spec, "v1.2.3")
            .expect_err("explicit directory")
            .contains("--dir")
    );
}

#[test]
fn objects() {
    use std::os::unix::ffi::OsStringExt as _;
    use std::os::unix::net::UnixListener;
    let fixture = tempfile::tempdir().expect("fixture");
    let root = fixture.path();
    write(root, "held", "data", 0o644);
    let alias = root.join("alias");
    std::os::unix::fs::symlink(root, &alias).expect("root alias");
    assert!(
        goods::directory(&alias)
            .expect_err("root link")
            .contains("plain directory")
    );
    std::fs::remove_file(alias).expect("unlink");
    let socket = root.join("socket");
    let _listener = UnixListener::bind(&socket).expect("socket");
    assert!(
        goods::directory(root)
            .expect_err("socket")
            .contains("plain file")
    );
    std::fs::remove_file(socket).expect("remove socket");
    let invalid = root.join(std::ffi::OsString::from_vec(vec![0xff]));
    std::fs::write(&invalid, "data").expect("non UTF-8");
    assert!(
        goods::directory(root)
            .expect_err("non UTF-8")
            .contains("UTF-8")
    );
    std::fs::remove_file(invalid).expect("remove invalid");
    write(root, "a\\b", "data", 0o644);
    assert!(
        goods::directory(root)
            .expect_err("backslash")
            .contains("anchored")
    );
}
