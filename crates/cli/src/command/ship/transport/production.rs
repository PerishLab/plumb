use crate::command::release::ReleaseMarker;
use plumb::rule::{Production, Receipt};
use sha2::{Digest, Sha256};

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Workload {
    target: String,
    archive: String,
    url: String,
    receipt: Receipt,
}

pub(super) fn materialize(
    root: &std::path::Path,
    workloads: &[Workload],
    marker: &ReleaseMarker,
) -> Result<(), String> {
    std::fs::create_dir_all(root)
        .map_err(|error| format!("cannot create {}: {error}", root.display()))?;
    for workload in workloads {
        contract(marker, &workload.target)?.verify(&workload.receipt)?;
        if workload.archive != marker.spec().target(&workload.target)?.archive {
            return Err("reused workload archive differs from its target".into());
        }
        if !workload.url.starts_with("https://") {
            return Err("binary workload must use an HTTPS URL".into());
        }
        let target = root.join(&workload.archive);
        let status = std::process::Command::new("curl")
            .args([
                "--fail",
                "--silent",
                "--show-error",
                "--location",
                "--retry",
                "3",
                "--output",
            ])
            .arg(&target)
            .arg(&workload.url)
            .status()
            .map_err(|error| format!("cannot fetch {}: {error}", workload.url))?;
        if !status.success() {
            return Err(format!(
                "cannot fetch binary workload for {}",
                workload.target
            ));
        }
        workload.receipt.verify(&target)?;
    }
    Ok(())
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
    } else {
        crate::command::workflow::plan::derive(input, Some(action))
    }
}

pub(super) fn contract(marker: &ReleaseMarker, triple: &str) -> Result<Production, String> {
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
        include_str!("../package.rs"),
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

pub(super) struct Input<'a> {
    pub target: &'a str,
    pub archive: &'a str,
    pub action: &'a str,
    pub keys: Option<&'a serde_json::Value>,
    pub contract: Option<&'a str>,
}

pub(super) fn execute(marker: &ReleaseMarker, input: Input<'_>) -> Result<(), String> {
    let contract = contract(marker, input.target)?;
    if input.contract != Some(contract.digest()?.as_str()) {
        return Err(
            "binary request production contract differs from its dispatch configuration and implementation".into(),
        );
    }
    if input.archive != marker.spec().target(input.target)?.archive {
        return Err("binary request archive differs from its target".into());
    }
    let keys = input
        .keys
        .ok_or("an exact ship request carries no inventory keys")?;
    let receipt = build(marker, input.target, &contract)?;
    let rig = plumb::rig::Rig::resolve(None).map_err(|error| error.to_string())?;
    crate::command::workflow::record::project(crate::command::workflow::record::Project {
        action: input.action,
        keys: &keys.to_string(),
        workload: crate::command::release::artifacts(&rig.release)?.join(input.archive),
        reuse: None,
        publication: None,
        depot: None,
        production: Some((&contract, receipt)),
    })
}

fn build(marker: &ReleaseMarker, triple: &str, contract: &Production) -> Result<Receipt, String> {
    let spec = marker.spec();
    crate::execution::inspect(
        "cargo",
        &spec.root,
        &plumb::config::environment(&contract.environment)?,
    )?;
    let producer = contract.start(&spec.root)?;
    let rig = plumb::rig::Rig::resolve(None).map_err(|error| error.to_string())?;
    let artifacts = crate::command::release::artifacts(&rig.release)?;
    let version = crate::command::release::channel::base(&marker.version)?;
    super::super::package::product(spec).produce(
        super::super::package::Build {
            target: triple,
            version: &version,
            channel: "stable",
            commit: &marker.commit,
            artifacts: &artifacts,
        },
        Some(producer.execution()),
    )?;
    let receipt = producer.finish(&artifacts.join(&spec.target(triple)?.archive))?;
    contract.verify(&receipt)?;
    Ok(receipt)
}
