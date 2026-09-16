use serde_json::{Value, json};

pub(super) fn attach(
    mut graph: Value,
    workflow: &toml::Table,
    atom: &str,
) -> Result<Value, String> {
    let controller = workflow
        .get("controller")
        .ok_or("locked workflow has no controller preparation")?;
    let targets = controller
        .get("targets")
        .ok_or("controller preparation has no tool worlds")?;
    let mut preparations = std::collections::BTreeMap::new();
    let nodes = graph["nodes"]
        .as_array_mut()
        .ok_or("Ship declaration has no nodes")?;
    for node in nodes.iter_mut() {
        let entry = node["execution"]["entry"]
            .as_str()
            .ok_or("Ship node has no entry")?
            .to_string();
        let world = targets
            .get(&entry)
            .ok_or_else(|| format!("controller has no tool world for {entry}"))?;
        let contract = json!({"schema": "plumb.controller-build/v1", "format": "gzip-6",
            "version": plumb::version!("PLUMB"), "channel": plumb::channel!("PLUMB"),
            "commit": atom,
            "target": text(world, "target")?, "rustc": text(world, "rustc")?, "cargo": text(world, "cargo")?,
            "environment": workflow.get("execution").and_then(|value| value.get("cargo"))
                .ok_or("controller requires the declared Cargo environment")?});
        let identity = format!("prepare/{entry}");
        preparations.insert(
            identity.clone(),
            preparation(&entry, &identity, contract, controller)?,
        );
        node["prepare"] = json!([{"node": identity, "output": "content"}]);
    }
    nodes.extend(preparations.into_values());
    Ok(graph)
}

fn preparation(
    entry: &str,
    identity: &str,
    contract: Value,
    controller: &toml::Value,
) -> Result<Value, String> {
    let source = controller
        .get("source")
        .ok_or("controller preparation has no source declaration")?;
    let implementation = controller
        .get("implementation")
        .ok_or("controller preparation has no implementation declaration")?;
    Ok(json!({"id": identity,
        "inputs": {"source": {"tree": source}, "implementation": {"tree": implementation},
            "contract": {"value": contract}}, "outputs": ["content"], "evidence": ["receipt"],
        "execution": {"entry": format!("controller.{entry}"), "capability": entry,
            "payload": contract, "materialize": ["source"]}}))
}

fn text<'a>(value: &'a toml::Value, name: &str) -> Result<&'a str, String> {
    value
        .get(name)
        .and_then(toml::Value::as_str)
        .ok_or_else(|| format!("controller tool world has no {name}"))
}
