#[allow(dead_code)]
#[path = "../../../src/command/guard/workflow/inventory.rs"]
mod inventory;
#[allow(dead_code)]
#[path = "../../../src/command/guard/workflow/remote.rs"]
mod remote;
#[allow(dead_code)]
#[path = "../../../src/command/guard/workflow/reuse.rs"]
mod reuse;

use plumb::rule::{Production, Receipt};
use reuse::{Inventory, Keys, Record};
use std::collections::BTreeMap;

#[test]
fn produced() {
    let root = tempfile::tempdir().unwrap();
    let workload = root.path().join("workload.tgz");
    std::fs::write(&workload, b"proven workload").unwrap();
    let mut receipt = receipt(&contract());
    receipt.artifact = inventory::digest(&workload).unwrap();
    let source = format!(
        "https://inventory.example/workloads/{}.tgz",
        receipt.artifact
    );
    assert!(inventory::produced(&receipt, &workload, None, &source).is_ok());
    assert!(inventory::produced(&receipt, &workload, Some(&source), &source).is_ok());
    for changed in [
        source.replace("inventory.example", "unverified.example"),
        source.replace(&receipt.artifact, &"c".repeat(64)),
        format!("{source}?unverified"),
        source.replace("https://", "http://"),
    ] {
        assert!(
            inventory::produced(&receipt, &workload, Some(&changed), &source)
                .unwrap_err()
                .contains("canonical workload URL")
        );
    }
    std::fs::write(&workload, b"changed workload").unwrap();
    assert!(
        inventory::produced(&receipt, &workload, Some(&source), &source)
            .unwrap_err()
            .contains("differs from its receipt")
    );
}

fn contract() -> Production {
    Production {
        platform: "windows-x86_64".into(),
        target: "x86_64-pc-windows-msvc".into(),
        implementation: "fixture/v1".into(),
        environment: plumb::config::Contract {
            inherit: vec!["PATH".into()],
            managed: vec![],
            reject: vec![],
            bind: Default::default(),
        },
        probes: BTreeMap::from([(
            "rule://fixture/cargo".into(),
            vec![plumb::rule::Probe {
                argv: vec!["cargo".into(), "--version".into()],
                stdout: "cargo fixture\n".into(),
                platform: None,
            }],
        )]),
    }
}

fn receipt(contract: &Production) -> Receipt {
    Receipt {
        schema: "plumb.production-receipt/v1".into(),
        contract: contract.digest().unwrap(),
        platform: contract.platform.clone(),
        artifact: "a".repeat(64),
        environment: "b".repeat(64),
        tools: BTreeMap::from([(
            "cargo".into(),
            plumb::config::Tool {
                path: "Z:/unallocated/windows/cargo.exe".into(),
                digest: "c".repeat(64),
            },
        )]),
        observations: BTreeMap::from([("rule://fixture/cargo".into(), "cargo fixture\r\n".into())]),
    }
}

fn keys() -> Keys {
    Keys {
        workload: "d".repeat(64),
        proof: "e".repeat(64),
        publication: Some("f".repeat(64)),
    }
}

fn record(contract: &Production) -> Record {
    let receipt = receipt(contract);
    let mut record = Record::workload(
        "ship/binary.fixture".into(),
        &keys(),
        format!(
            "https://inventory.invalid/workloads/{}.tgz",
            receipt.artifact
        ),
    );
    record.receipt = Some(receipt);
    record
}

#[test]
fn historical() {
    let contract = contract();
    let mut inventory = Inventory::empty();
    inventory.records.push(record(&contract));
    let (verdict, receipt) = inventory
        .verified("ship/binary.fixture", &keys(), Some(&contract))
        .unwrap();
    assert_eq!(verdict.reason, "publication-moved");
    assert_eq!(verdict.source.kind, "workload");
    assert!(
        receipt.is_some(),
        "remote evidence is verified without allocating its host"
    );
    inventory.records[0].proof = Some("0".repeat(64));
    let (verdict, receipt) = inventory
        .verified("ship/binary.fixture", &keys(), Some(&contract))
        .unwrap();
    assert_eq!(verdict.reason, "proof-moved");
    assert_eq!(verdict.decision, "run");
    assert!(receipt.is_none());
}

#[test]
fn incomplete() {
    let contract = contract();
    let full = record(&contract);
    let mut legacy = full.clone();
    legacy.receipt = None;
    let mut missing = full.clone();
    missing.receipt.as_mut().unwrap().tools.clear();
    let mut stdout = full.clone();
    stdout.receipt.as_mut().unwrap().observations.clear();
    let mut source = full.clone();
    source.source.source = "https://inventory.invalid/workloads/wrong.tgz".into();
    let mut moved = full;
    moved.receipt.as_mut().unwrap().contract = "0".repeat(64);
    for record in [legacy, missing, stdout, source, moved] {
        let mut inventory = Inventory::empty();
        inventory.records.push(record);
        let (verdict, receipt) = inventory
            .verified("ship/binary.fixture", &keys(), Some(&contract))
            .unwrap();
        assert_eq!(verdict.reason, "record-absent");
        assert_eq!(verdict.source.kind, "none");
        assert!(receipt.is_none());
    }
}

#[test]
fn routes() {
    let record = record(&contract());
    for (_, record) in record.routes() {
        let body = record.encode().unwrap();
        let decoded = Record::decode(&body).unwrap();
        assert_eq!(decoded, record);
        assert!(decoded.receipt.is_some());
    }
}

#[test]
fn producers() {
    let contract = contract();
    let historical = record(&contract);
    let mut current = historical.clone();
    let receipt = current.receipt.as_mut().unwrap();
    receipt.tools.get_mut("cargo").unwrap().path = "C:/another/host/cargo.exe".into();
    receipt.environment = "0".repeat(64);
    assert!(historical.equivalent(&current, Some(&contract)));
    assert!(!historical.equivalent(&current, None));
    current.receipt.as_mut().unwrap().artifact = "1".repeat(64);
    assert!(!historical.equivalent(&current, Some(&contract)));
}

#[test]
fn binding() {
    let binding = inventory::Binding::new(&"a".repeat(64), "oci://registry.test/probe").unwrap();
    let other = inventory::Binding::new(&"b".repeat(64), "oci://registry.test/probe").unwrap();
    assert_ne!(binding.key, other.key);
    let mut held = Record::publication(
        "ship/oci".into(),
        &keys(),
        "https://registry.test/digest".into(),
        None,
    )
    .unwrap();
    held.binding = Some(binding.key.clone());
    let mut current = held.clone();
    current.workload = "1".repeat(64);
    current.proof = Some("2".repeat(64));
    current.publication = Some("3".repeat(64));
    assert!(held.binding(&current).is_ok());
    let routes = held.routes();
    assert_eq!(routes[0].0, format!("records/binding/{}.json", binding.key));
    let moved = current.routes();
    assert_eq!(routes[0].0, moved[0].0);
    assert_ne!(routes[1].0, moved[1].0);
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("inventory.json");
    let mut inventory = Inventory::empty();
    inventory.records = vec![held.clone(), current.clone()];
    std::fs::write(&path, serde_json::to_vec(&inventory).unwrap()).unwrap();
    assert_eq!(
        binding.resolve(Some(&path), None).unwrap(),
        Some(held.source.source.clone())
    );
    assert_eq!(other.resolve(Some(&path), None).unwrap(), None);
    current.source.source = "https://registry.test/different".into();
    assert!(held.binding(&current).unwrap_err().contains("drifted"));
    inventory.records.push(current);
    std::fs::write(&path, serde_json::to_vec(&inventory).unwrap()).unwrap();
    assert!(
        binding
            .resolve(Some(&path), None)
            .unwrap_err()
            .contains("ambiguous")
    );
    held.source.kind = "workload".into();
    assert!(held.valid().is_err());
}

#[test]
fn permanent() {
    let binding = inventory::Binding::new(&"a".repeat(64), "oci://registry.test/probe").unwrap();
    let store = crate::support::Bucket::open(3);
    let url = format!("{}/workflow/inventory.json", store.endpoint());
    assert_eq!(binding.resolve(None, Some(&url)).unwrap(), None);
    let mut record = Record::publication(
        "ship/oci".into(),
        &keys(),
        "https://registry.test/digest".into(),
        None,
    )
    .unwrap();
    record.binding = Some(binding.key.clone());
    let route = record.routes()[0].0.clone();
    store.seed(&route, &record.encode().unwrap());
    assert_eq!(
        binding.resolve(None, Some(&url)).unwrap(),
        Some(record.source.source.clone())
    );
    record.binding = Some("f".repeat(64));
    store.seed(&route, &record.encode().unwrap());
    assert!(
        binding
            .resolve(None, Some(&url))
            .unwrap_err()
            .contains("different identity")
    );
    store.finish();
    assert!(
        binding.resolve(None, Some(&url)).is_err(),
        "unreadable authority cannot prove absence"
    );
}
