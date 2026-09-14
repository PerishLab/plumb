use crate::command::release::ReleaseMarker;
use plumb::rule::{Production, Receipt};
use sha2::{Digest, Sha256};

pub(super) fn reuse(action: &str, node: &mut serde_json::Value) {
    if action == "ship/oci" && node["receipt"].is_null() && node["reuse"]["type"] == "workload" {
        node["reuse"] = serde_json::json!({ "type": "none", "source": "" });
    }
}

pub(super) fn plan(
    input: crate::command::workflow::plan::Input,
    marker: &ReleaseMarker,
    action: &str,
    target: Option<&str>,
) -> Result<String, String> {
    if action.starts_with("ship/binary.") {
        let contract = contract(marker, target.ok_or("binary plan has no target")?)?;
        crate::command::workflow::plan::production(input, action, &contract)
    } else if action == "ship/oci" {
        let contract = super::super::package::production::contract(marker)?;
        crate::command::workflow::plan::production(input, action, &contract)
    } else {
        crate::command::workflow::plan::derive(input, Some(action))
    }
}

pub(in crate::command::ship) fn contract(
    marker: &ReleaseMarker,
    triple: &str,
) -> Result<Production, String> {
    let governance = crate::shape::product::resolve(&marker.spec().root, "")?;
    let profile = governance
        .profile
        .as_ref()
        .ok_or("binary production requires the dispatch configuration's Product Profile")?;
    let workflow = crate::catalog::set::read("workflow")?;
    let execution = workflow
        .get("execution")
        .ok_or("workflow rules declare no execution contract")?;
    let environment = crate::execution::contract("cargo")?;
    let runner = &marker.spec().target(triple)?.runner;
    let platform = execution
        .get("platform")
        .and_then(|held| held.get(runner))
        .and_then(toml::Value::as_str)
        .ok_or_else(|| format!("workflow rules declare no platform for {runner}"))?;
    let roots = super::support::sources(marker.spec())?;
    let probes = crate::catalog::probe::read(&profile.manifest, roots.iter().map(String::as_str))?;
    let contract = Production {
        platform: platform.to_string(),
        target: triple.to_string(),
        implementation: implementation(),
        environment,
        probes,
    };
    contract.digest()?;
    Ok(contract)
}

fn implementation() -> String {
    let mut digest = Sha256::new();
    for source in [
        include_str!("production.rs"),
        include_str!("../package/mod.rs"),
        include_str!("../archive.rs"),
        include_str!("../../release/workspace.rs"),
        include_str!("../../../execution/cargo.rs"),
        include_str!("../../../execution/mod.rs"),
        include_str!("../../../catalog/probe.rs"),
        include_str!("../../../../../lib/src/proof/rule/production.rs"),
        include_str!("../../../../../lib/src/proof/rule/probe.rs"),
        include_str!("../../../../../lib/src/proof/rule/process.rs"),
        include_str!("../../../../../lib/src/runtime/environment.rs"),
        include_str!("../../../../../lib/src/runtime/cache.rs"),
        include_str!("../../../../../lib/src/runtime/execution.rs"),
        include_str!("../../../../../lib/src/runtime/process.rs"),
    ] {
        let source = source.replace("\r\n", "\n");
        digest.update((source.len() as u64).to_le_bytes());
        digest.update(source.as_bytes());
    }
    format!("{:x}", digest.finalize())
}

pub(in crate::command::ship) fn produce(
    marker: &ReleaseMarker,
    triple: &str,
    artifacts: &std::path::Path,
) -> Result<Receipt, String> {
    let contract = contract(marker, triple)?;
    let spec = marker.spec();
    crate::execution::inspect(
        "cargo",
        &spec.root,
        &plumb::config::environment(&contract.environment)?,
    )?;
    let producer = contract.start(&spec.root)?;
    let version = crate::command::release::channel::base(&marker.version)?;
    super::super::package::product(spec).produce(
        super::super::package::Build {
            target: triple,
            version: &version,
            channel: "stable",
            commit: &marker.commit,
            artifacts,
        },
        Some(producer.execution()),
    )?;
    let receipt = producer.finish(&artifacts.join(&spec.target(triple)?.archive))?;
    contract.verify(&receipt)?;
    Ok(receipt)
}
