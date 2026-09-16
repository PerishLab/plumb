use crate::command::release::ReleaseMarker;
use serde_json::{Value, json};

pub(super) fn graph(marker: &ReleaseMarker, atom: &str) -> Result<Value, String> {
    marker.spec().ship()?;
    let workflow = crate::catalog::set::read("workflow")?;
    let inputs = workflow
        .get("inputs")
        .and_then(|value| value.get(&marker.product))
        .ok_or("locked workflow declares no product inputs")?;
    let implementation = workflow
        .get("implementations")
        .ok_or("locked workflow declares no implementations")?;
    let mut graph = Graph {
        marker,
        implementation,
        nodes: Vec::new(),
        targets: Vec::new(),
    };
    if marker.spec().binary() {
        graph.binary(inputs)?;
    }
    graph.publications(inputs)?;
    graph.complete()?;
    super::controllers::attach(
        json!({"schema": "plumb.blob-graph/v1", "nodes": graph.nodes, "targets": graph.targets}),
        &workflow,
        atom,
    )
}

struct Graph<'a> {
    marker: &'a ReleaseMarker,
    implementation: &'a toml::Value,
    nodes: Vec<Value>,
    targets: Vec<String>,
}

impl Graph<'_> {
    fn complete(&mut self) -> Result<(), String> {
        let requires = self
            .targets
            .iter()
            .map(|name| json!({"node": name, "output": "content"}))
            .collect::<Vec<_>>();
        let resources = self
            .targets
            .iter()
            .map(|name| (name.clone(), Value::Null))
            .collect::<serde_json::Map<_, _>>();
        let action = "ship/complete";
        let mut completion = node(
            action,
            json!({"implementation": self.implementation("complete")?,
            "identity": self.marker.digest()?}),
            self.request(action, json!({"type": "complete", "resources": resources})),
            "complete",
        );
        completion["requires"] = json!(requires);
        self.nodes.push(completion);
        self.targets = vec![action.into()];
        Ok(())
    }

    fn binary(&mut self, inputs: &toml::Value) -> Result<(), String> {
        let source = input(inputs, "binary")?;
        let mut publication = serde_json::Map::new();
        let mut workloads = Vec::new();
        for target in &self.marker.spec().target {
            let producer = format!("ship/produce.{}", target.triple);
            let binding = format!("ship/binary.{}", target.triple);
            let production = super::production::contract(self.marker, &target.triple)?.digest()?;
            let proof = crate::command::ship::native::proof::contract(self.marker, &target.triple)?
                .digest()?;
            let mut payload = self.request(
                &producer,
                json!({
                    "type": "produce", "target": target.triple, "archive": target.archive,
                }),
            );
            payload["production"] = json!(production);
            self.nodes.push(node(
                &producer,
                json!({
                    "implementation": self.implementation("produce")?, "production": production,
                    "source": {"tree": source},
                }),
                payload,
                &format!("produce.{}", target.runner),
            ));
            let build = json!({"production": production, "reuse": {"type": "none", "source": ""}, "receipt": null});
            let mut payload = self.request(
                &binding,
                json!({
                    "type": "bind", "target": target.triple, "archive": target.archive,
                    "build": build,
                }),
            );
            payload["production"] = json!(proof);
            self.nodes.push(node(&binding, json!({
                "implementation": self.implementation("bind")?, "production": proof, "identity": self.marker.digest()?,
                "content": {"node": producer, "output": "content"},
            }), payload, &format!("bind.{}", target.runner)));
            publication.insert(
                target.triple.clone(),
                json!({"node": binding, "output": "content"}),
            );
            workloads.push(json!({"target": target.triple, "archive": target.archive}));
        }
        publication.insert("implementation".into(), self.implementation("publish")?);
        publication.insert("identity".into(), json!(self.marker.digest()?));
        let action = "ship/binary";
        self.nodes.push(node(
            action,
            Value::Object(publication),
            self.request(
                action,
                json!({"type": "publication", "workloads": workloads}),
            ),
            "publish",
        ));
        self.targets.push(action.into());
        Ok(())
    }

    fn publications(&mut self, inputs: &toml::Value) -> Result<(), String> {
        let surface: Value =
            serde_json::from_str(&crate::command::release::plan::surface(self.marker.spec())?)
                .map_err(|error| error.to_string())?;
        let rows = surface["publication"]["include"]
            .as_array()
            .ok_or("publication surface is absent")?;
        for row in rows {
            let action = row["action"]
                .as_str()
                .ok_or("publication action is absent")?;
            let source = input(inputs, action)?;
            let mut payload = self.request(action, row["operation"].clone());
            let mut references = json!({"implementation": self.implementation("publish")?,
                "identity": self.marker.digest()?});
            if action == "ship/oci" {
                references["source"] = json!({"tree": source});
                self.image(&mut payload, &mut references)?;
            } else {
                references["content"] = self.package(action, source, &row["operation"])?;
            }
            self.nodes
                .push(node(action, references, payload, "publish"));
            self.targets.push(action.into());
        }
        Ok(())
    }

    fn image(&self, payload: &mut Value, references: &mut Value) -> Result<(), String> {
        let production = super::super::package::production::contract(self.marker)?.digest()?;
        payload["production"] = json!(production);
        references["production"] = json!(production);
        let mut workloads = Vec::new();
        for target in &self.marker.spec().target {
            if !target.triple.contains("linux") {
                continue;
            }
            references[&target.triple] =
                json!({"node": format!("ship/binary.{}", target.triple), "output": "content"});
            workloads.push(json!({"target": target.triple, "archive": target.archive}));
        }
        payload["operation"]["workloads"] = json!(workloads);
        Ok(())
    }

    fn package(&mut self, action: &str, source: Value, operation: &Value) -> Result<Value, String> {
        let producer = format!("ship/produce.{}", action.trim_start_matches("ship/"));
        let mut inputs = json!({"implementation": self.implementation("package")?,
            "operation": {"value": operation}, "source": {"tree": source}});
        if action == "ship/cfworker" {
            inputs["identity"] =
                json!({"value": {"commit": self.marker.commit, "version": self.marker.version}});
        }
        self.nodes.push(node(
            &producer,
            inputs,
            self.request(
                &producer,
                json!({"type": "package", "operation": operation}),
            ),
            "produce.linux",
        ));
        Ok(json!({"node": producer, "output": "content"}))
    }

    fn request(&self, action: &str, operation: Value) -> Value {
        json!({"schema": "plumb.ship-request/v3", "marker": self.marker.marker,
            "action": action, "operation": operation,
            "configuration": self.marker.spec().configuration, "profile": self.marker.spec().profile})
    }

    fn implementation(&self, entry: &str) -> Result<Value, String> {
        Ok(json!({"tree": input(self.implementation, entry)?}))
    }
}

fn input(inputs: &toml::Value, action: &str) -> Result<Value, String> {
    let selected = inputs
        .get(action)
        .ok_or_else(|| format!("locked workflow has no input declaration for {action}"))?;
    serde_json::to_value(selected).map_err(|error| error.to_string())
}

fn node(action: &str, inputs: Value, payload: Value, entry: &str) -> Value {
    let materialize = if matches!(
        payload["operation"]["type"].as_str(),
        Some("produce" | "package")
    ) {
        vec!["source"]
    } else {
        Vec::new()
    };
    json!({"id": action, "inputs": inputs, "outputs": ["content"], "evidence": ["receipt"],
        "execution": {"entry": entry, "capability": entry, "payload": payload, "materialize": materialize}})
}
