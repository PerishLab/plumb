use crate::command::release::ReleaseMarker;
use plumb::rule::{Production, Receipt};
use sha2::{Digest, Sha256};

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
    input: &std::path::Path,
) -> Result<Receipt, String> {
    let contract = contract(marker, triple)?;
    let spec = marker.spec();
    let input = input.canonicalize().map_err(|error| error.to_string())?;
    if !input.is_dir()
        || input.join(".git").exists()
        || input
            == spec
                .root
                .canonicalize()
                .map_err(|error| error.to_string())?
    {
        return Err("production requires an isolated materialized input directory".into());
    }
    crate::execution::inspect(
        "cargo",
        &input,
        &plumb::config::environment(&contract.environment)?,
    )?;
    let producer = contract.start(&input)?;
    let workspace =
        crate::command::release::workspace::Workspace::bound(producer.execution(), &input)?;
    let binaries = workspace.build(
        spec,
        crate::command::release::workspace::Build {
            root: &input,
            triple,
            version: "v0.0.0",
            commit: "",
        },
        Some(producer.execution()),
    )?;
    std::fs::create_dir_all(artifacts).map_err(|error| error.to_string())?;
    let target = spec.target(triple)?;
    let path = artifacts.join(&target.archive);
    if path.exists() {
        return Err("production output already exists".into());
    }
    super::super::archive::write(target.format, &path, &binaries)?;
    let mut receipt = producer.finish(&path)?;
    receipt.source = Some(plumb::rule::Source {
        commit: marker.commit.clone(),
        tree: marker.tree.clone(),
    });
    contract.verify(&receipt)?;
    Ok(receipt)
}
