use plumb::config::Contract;
use plumb::rule::{Probe, Production};
use std::collections::BTreeMap;

fn contract() -> Production {
    let output = std::process::Command::new("git")
        .arg("--version")
        .output()
        .expect("git");
    assert!(output.status.success());
    Production {
        platform: plumb::config::platform(),
        target: "fixture".into(),
        implementation: "fixture/v1".into(),
        environment: Contract {
            inherit: ["PATH", "SystemRoot", "SYSTEMROOT", "WINDIR", "PATHEXT"]
                .map(str::to_string)
                .to_vec(),
            managed: vec![],
            reject: vec![],
        },
        probes: BTreeMap::from([(
            "rule://fixture/git".into(),
            vec![Probe {
                argv: vec!["git".into(), "--version".into()],
                stdout: String::from_utf8(output.stdout).expect("version"),
                platform: None,
            }],
        )]),
    }
}

#[test]
fn receipt() {
    let held = contract();
    let root = tempfile::tempdir().expect("root");
    let artifact = root.path().join("artifact");
    std::fs::write(&artifact, b"artifact").expect("artifact");
    let producer = held.start(root.path()).expect("producer");
    assert!(producer.execution().command("git").is_ok());
    let receipt = producer.finish(&artifact).expect("receipt");
    held.verify(&receipt).expect("contract");
    receipt.verify(&artifact).expect("artifact");
    let mut remote = receipt.clone();
    remote.tools.get_mut("git").unwrap().path = "Z:/historical/producer/git.exe".into();
    held.verify(&remote)
        .expect("historical path is evidence, not current host identity");
    remote
        .observations
        .insert("rule://fixture/git".into(), "different".into());
    assert!(held.verify(&remote).is_err());
    let mut missing = receipt.clone();
    missing.tools.clear();
    assert!(held.verify(&missing).is_err());
    let mut missing = receipt.clone();
    missing.observations.clear();
    assert!(held.verify(&missing).is_err());
    std::fs::write(&artifact, b"drift").expect("drift");
    assert!(receipt.verify(&artifact).is_err());
}

#[test]
fn identity() {
    let mut held = contract();
    let original = held.digest().expect("identity");
    held.target.push_str("-other");
    assert_ne!(original, held.digest().unwrap());
    held.target = "fixture".into();
    held.implementation.push_str("-other");
    assert_ne!(original, held.digest().unwrap());
    held.implementation = "fixture/v1".into();
    held.probes.get_mut("rule://fixture/git").unwrap()[0]
        .stdout
        .push_str("-other");
    assert_ne!(original, held.digest().unwrap());
    let root = tempfile::tempdir().expect("root");
    assert!(held.start(root.path()).is_err());
}

#[test]
fn platform() {
    let mut held = contract();
    held.platform = "unallocated-host".into();
    held.digest()
        .expect("planning does not probe a remote host");
    let root = tempfile::tempdir().expect("root");
    assert!(held.start(root.path()).is_err());
    held.probes.clear();
    assert!(held.digest().is_err());
}

#[test]
#[cfg(unix)]
fn changed() {
    let mut held = contract();
    let root = tempfile::tempdir().expect("root");
    let tool = root.path().join("git");
    let original = which::which("git").expect("git path");
    std::fs::copy(original, &tool).expect("copy tool");
    held.probes.get_mut("rule://fixture/git").unwrap()[0].argv[0] =
        tool.to_string_lossy().into_owned();
    let producer = held.start(root.path()).expect("producer");
    let artifact = root.path().join("artifact");
    std::fs::write(&artifact, b"artifact").expect("artifact");
    std::fs::write(&tool, b"changed tool").expect("replace tool");
    assert!(producer.finish(&artifact).unwrap_err().contains("changed"));
}
