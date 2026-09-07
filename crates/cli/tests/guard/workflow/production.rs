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
