use crate::shape::release::Spec;
use plumb::config::{Contract, Execution};
use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

#[path = "client.rs"]
mod client;
#[path = "native.rs"]
mod native;

fn execution(root: &Path, values: BTreeMap<String, String>) -> Execution {
    let contract = Contract {
        inherit: values.keys().cloned().collect(),
        managed: Vec::new(),
        reject: Vec::new(),
        bind: BTreeMap::new(),
    };
    let environment = contract
        .capture(
            values
                .into_iter()
                .map(|(key, value)| (key.into(), value.into())),
        )
        .unwrap();
    Execution::new(environment, &["regctl".into()], root).unwrap()
}

fn spec(root: &Path, registry: &str, repository: &str, account: &str) -> Spec {
    Spec::decode(root, &format!(
        "[release]\nproduct='probe'\nauthority='https://releases.test'\n[release.oci]\nregistry={registry:?}\nimage={repository:?}\naccount={account:?}\n"
    ), "fixture").unwrap()
}

fn executable(path: &Path, body: &str) {
    std::fs::write(path, body).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn publication() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let spec = spec(root, "registry.example", "owner/probe", "Example");
    let source = root.join("source.tar");
    archive(&source, 1);
    let observed = root.join("calls");
    let scenario = root.join("scenario");
    let ambient = root.join("ambient");
    std::fs::write(&ambient, "untouched").unwrap();
    executable(&root.join("regctl"), client::CLIENT);
    let execution = execution(
        root,
        BTreeMap::from([
            (
                "PATH".into(),
                format!("{}:{}", root.display(), std::env::var("PATH").unwrap()),
            ),
            ("PLUMB_TEST_CALLS".into(), observed.display().to_string()),
            ("PLUMB_TEST_SCENARIO".into(), scenario.display().to_string()),
            ("REGCTL_CONFIG".into(), ambient.display().to_string()),
            ("DOCKER_AUTH_CONFIG".into(), "ambient credentials".into()),
        ]),
    );
    let publish = || {
        let image = super::Image::read(&spec, &source, Some(&execution))?;
        assert_eq!(image.provenance, "c".repeat(40));
        image.publish(&spec, "Bearer secret")
    };
    let check = || {
        let calls = std::fs::read_to_string(&observed).unwrap();
        assert!(!calls.contains("secret"), "credential entered argv");
        assert!(!calls.contains("docker "), "{calls}");
        for path in calls.lines().filter_map(|line| line.strip_prefix("seat ")) {
            assert!(!Path::new(path).exists(), "configuration leaked: {path}");
        }
        assert_eq!(std::fs::read_to_string(&ambient).unwrap(), "untouched");
        calls
    };
    std::fs::write(&scenario, "normal").unwrap();
    let expected = format!(
        "https://registry.example/v2/owner/probe/manifests/sha256:{}1",
        "0".repeat(63)
    );
    for _ in 0..2 {
        std::fs::write(&observed, "").unwrap();
        assert_eq!(publish().unwrap(), expected);
        let calls = check();
        assert!(
            calls.contains("registry.example/owner/probe:sha256-"),
            "{calls}"
        );
        assert!(calls.contains("--force-recursive"), "{calls}");
    }
    for (case, expected) in [
        ("invalid", "no valid manifest digest"),
        ("provenance", "no valid org.opencontainers.image.revision"),
        ("login", "image attachment login failed"),
        ("drift", "published image drift"),
        ("anonymous", "cannot be read anonymously"),
        ("readback", "readback changed content identity"),
    ] {
        std::fs::write(&scenario, case).unwrap();
        std::fs::write(&observed, "").unwrap();
        let error = publish().unwrap_err();
        assert!(error.contains(expected), "{case}: {error}");
        check();
    }
    archive(&source, 2);
    assert!(publish().unwrap_err().contains("exactly one image"));
}

fn archive(path: &Path, count: usize) {
    let file = std::fs::File::create(path).unwrap();
    let mut archive = tar::Builder::new(file);
    let body =
        serde_json::to_vec(&vec![serde_json::json!({"Config":"config.json"}); count]).unwrap();
    let mut header = tar::Header::new_gnu();
    header.set_mode(0o644);
    header.set_size(body.len() as u64);
    header.set_cksum();
    archive
        .append_data(&mut header, "manifest.json", body.as_slice())
        .unwrap();
    archive.finish().unwrap();
}
