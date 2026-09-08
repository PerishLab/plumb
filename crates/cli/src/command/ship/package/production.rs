use crate::command::release::ReleaseMarker;
use plumb::rule::{Probe, Production, Receipt};
use sha2::{Digest, Sha256};

pub(in crate::command::ship) struct Proof<'a> {
    pub contract: &'a Production,
    pub receipt: Option<&'a Receipt>,
}

pub(in crate::command::ship) fn contract(marker: &ReleaseMarker) -> Result<Production, String> {
    let governance = crate::shape::product::resolve(&marker.spec().root, "")?;
    let profile = governance
        .profile
        .as_ref()
        .ok_or("image production requires a Product Profile")?;
    let mut probes = crate::catalog::probe::read(&profile.manifest, ["Containerfile"])?;
    let workflow = crate::catalog::set::read("workflow")?;
    let platform = workflow
        .get("execution")
        .and_then(|held| held.get("platform"))
        .and_then(|held| held.get("linux"))
        .and_then(toml::Value::as_str)
        .ok_or("workflow declares no image execution platform")?;
    for rules in probes.values() {
        Probe::select(rules, platform)?;
    }
    probes.retain(|_, rules| {
        Probe::select(rules, platform).is_ok_and(|probe| probe.argv[0] != "regctl")
    });
    let contract = Production {
        platform: platform.to_string(),
        target: "linux/amd64".into(),
        implementation: implementation(),
        environment: crate::execution::contract("oci")?,
        probes,
    };
    contract.digest()?;
    for program in ["docker"] {
        if !contract
            .probes
            .values()
            .any(|rules| Probe::select(rules, platform).is_ok_and(|probe| probe.argv[0] == program))
        {
            return Err(format!("image production requires a {program} probe"));
        }
    }
    Ok(contract)
}

fn implementation() -> String {
    let mut digest = Sha256::new();
    for source in [
        include_str!("production.rs"),
        include_str!("context.rs"),
        include_str!("../adaptor/image.rs"),
        include_str!("../../../catalog/probe.rs"),
        include_str!("../../../../../lib/src/proof/rule/production.rs"),
        include_str!("../../../../../lib/src/runtime/execution.rs"),
    ] {
        let source = source.replace("\r\n", "\n");
        digest.update((source.len() as u64).to_le_bytes());
        digest.update(source.as_bytes());
    }
    format!("{:x}", digest.finalize())
}
