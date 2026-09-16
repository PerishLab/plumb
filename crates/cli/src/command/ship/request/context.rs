use serde_json::Value;
use sha2::{Digest, Sha256};

pub(super) fn request(context: &Value) -> Result<Value, String> {
    let mut request = context["payload"].clone();
    if request["schema"] != "plumb.ship-request/v3" || request["action"] != context["node"] {
        return Err("execution payload differs from its Ship action".into());
    }
    match request["operation"]["type"].as_str() {
        Some("produce" | "package") => {
            let input = &context["materialized"]["source"];
            let expected = text(&context["inputs"]["source"], "digest")?;
            if text(input, "key")? != expected {
                return Err("production input differs from its execution context".into());
            }
            request["input"] = Value::String(text(input, "root")?.to_string());
        }
        Some("bind") => {
            let target = text(&request["operation"], "target")?;
            let producer = format!("ship/produce.{target}");
            let source = &context["producers"][&producer];
            let content = &source["outputs"]["content"];
            if text(content, "digest")? != text(&context["inputs"]["content"], "digest")? {
                return Err("binding content differs from its producer".into());
            }
            request["operation"]["build"]["reuse"] = content["reuse"].clone();
            request["operation"]["build"]["receipt"] = proof(&source["evidence"]["receipt"])?;
        }
        Some("complete") => {
            let names = request["operation"]["resources"]
                .as_object()
                .ok_or("completion request has no resource set")?
                .keys()
                .cloned()
                .collect::<Vec<_>>();
            for name in names {
                request["operation"]["resources"][&name] =
                    proof(&context["producers"][&name]["evidence"]["receipt"])?;
            }
        }
        Some("publication" | "oci") => {
            request["operation"]["workloads"] = workloads(context, &request)?;
        }
        _ => {
            let action = text(&request, "action")?
                .strip_prefix("ship/")
                .ok_or("publication action has no Ship namespace")?;
            let producer = format!("ship/produce.{action}");
            let source = &context["producers"][&producer]["outputs"]["content"];
            if text(source, "digest")? != text(&context["inputs"]["content"], "digest")? {
                return Err("publication workload differs from its producer".into());
            }
            request["reuse"] = source["reuse"].clone();
        }
    }
    Ok(request)
}

fn workloads(context: &Value, request: &Value) -> Result<Value, String> {
    let workloads = request["operation"]["workloads"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut resolved = Vec::new();
    for workload in workloads {
        let target = text(&workload, "target")?;
        let producer = format!("ship/binary.{target}");
        let source = &context["producers"][&producer];
        let content = &source["outputs"]["content"];
        if text(content, "digest")? != text(&context["inputs"][target], "digest")? {
            return Err("publication content differs from its producer".into());
        }
        resolved.push(
            serde_json::json!({"target": target, "archive": workload["archive"],
            "url": text(&content["reuse"], "source")?,
            "receipt": proof(&source["evidence"]["receipt"])?}),
        );
    }
    Ok(Value::Array(resolved))
}

fn proof(reference: &Value) -> Result<Value, String> {
    let digest = text(reference, "digest")?;
    let reuse = &reference["reuse"];
    let source = text(reuse, "source")?;
    if reuse["type"] != "workload" || !source.starts_with("https://") {
        return Err("production evidence requires a verified blob reference".into());
    }
    let bytes = plumb::bucket::fetch(source)?.ok_or("production evidence is absent")?;
    if format!("{:x}", Sha256::digest(&bytes)) != digest {
        return Err("production evidence blob digest mismatch".into());
    }
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}

fn text<'a>(value: &'a Value, field: &str) -> Result<&'a str, String> {
    value[field]
        .as_str()
        .filter(|held| !held.is_empty())
        .ok_or_else(|| format!("execution context has no {field}"))
}
