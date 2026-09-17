use serde_json::{Value, json};

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Configuration {
    marker: plumb::depot::v3::Marker,
    generation: String,
}

impl Configuration {
    pub(super) fn held() -> Result<Option<Self>, String> {
        let root = plumb::depot::root(std::path::Path::new(""))?;
        let path = root.join(plumb::depot::v3::POINTER);
        let bytes = std::fs::read(&path)
            .map_err(|error| format!("cannot read controller configuration: {error}"))?;
        let value: Value = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        if value["format"] != 3 {
            return Ok(None);
        }
        let pointer = plumb::depot::v3::Pointer::parse(&bytes)?;
        if pointer.generation != plumb::depot::rules()?.mark() {
            return Err("controller configuration differs from the selected generation".into());
        }
        Ok(Some(Self {
            marker: pointer.marker,
            generation: pointer.generation,
        }))
    }

    pub(super) fn select(&self, root: &std::path::Path) -> Result<(), String> {
        crate::command::depot::candidate::select(root, &self.marker.name, &self.generation)?;
        if crate::command::depot::candidate::selected().map(|pointer| &pointer.marker)
            != Some(&self.marker)
        {
            return Err("controller configuration marker differs from its request".into());
        }
        Ok(())
    }
}

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
    let configuration = Configuration::held()?;
    let nodes = graph["nodes"]
        .as_array_mut()
        .ok_or("Ship declaration has no nodes")?;
    for node in nodes.iter_mut() {
        if let Some(configuration) = &configuration {
            node["execution"]["payload"]["controller"] =
                serde_json::to_value(configuration).map_err(|error| error.to_string())?;
        }
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
        let mut prepared = preparation(&entry, &identity, contract, controller)?;
        node["prepare"] = json!([{"node": identity, "output": "content"}]);
        if let Some(runtime) = workflow.get("runtime") {
            let bundle = runtime
                .get("targets")
                .and_then(|value| value.get(text(world, "target").ok()?))
                .ok_or("runtime preparation has no matching target")?;
            let dependency = format!("runtime/{entry}");
            activate(&mut prepared, &dependency, bundle)?;
            let operation = &node["execution"]["payload"]["operation"];
            let operation = if operation["type"] == "package" {
                &operation["operation"]
            } else {
                operation
            };
            let required = runtime
                .get("operations")
                .and_then(toml::Value::as_array)
                .ok_or("runtime requires explicit operation selection")?;
            if required
                .iter()
                .any(|value| value.as_str() == operation["type"].as_str())
            {
                activate(node, &dependency, bundle)?;
            }
            preparations.insert(
                dependency.clone(),
                requirement(&entry, &dependency, bundle, runtime)?,
            );
        }
        preparations.insert(identity, prepared);
    }
    nodes.extend(preparations.into_values());
    Ok(graph)
}

fn activate(node: &mut Value, identity: &str, bundle: &toml::Value) -> Result<(), String> {
    node["inputs"]["runtime"] = json!({"value": bundle});
    if node.get("prepare").is_none() {
        node["prepare"] = json!([]);
    }
    node["prepare"]
        .as_array_mut()
        .ok_or("runtime requires a preparation list")?
        .push(json!({"node": identity, "output": "content"}));
    Ok(())
}

fn requirement(
    entry: &str,
    identity: &str,
    bundle: &toml::Value,
    runtime: &toml::Value,
) -> Result<Value, String> {
    let implementation = runtime
        .get("implementation")
        .ok_or("runtime preparation has no implementation declaration")?;
    Ok(json!({"id": identity,
        "inputs": {"implementation": {"tree": implementation}, "contract": {"value": bundle}},
        "outputs": ["content"], "evidence": ["receipt"],
        "execution": {"entry": format!("runtime.{entry}"), "capability": entry, "payload": bundle}}))
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
