use super::channel;
use super::model::Spec;
use crate::command::operator::topology;

pub fn plan(spec: &Spec, reference: &str, commit: &str) -> Result<String, String> {
    let version = topology::reference(reference)?;
    let channel = channel::channel(&version)?;
    topology::bind(topology::Source {
        root: &spec.root,
        channel: &channel,
        version: &version,
        commit,
        reference,
    })?;
    let mut held = media(spec)?;
    let table = held.as_object_mut().ok_or("media is not an object")?;
    table.insert("version".into(), serde_json::json!(version));
    table.insert("channel".into(), serde_json::json!(channel));
    table.insert("authority".into(), serde_json::json!(spec.authority));
    serde_json::to_string(&held).map_err(|error| error.to_string())
}

pub fn surface(spec: &Spec) -> Result<String, String> {
    serde_json::to_string(&media(spec)?).map_err(|error| error.to_string())
}

fn media(spec: &Spec) -> Result<serde_json::Value, String> {
    let held = spec.surface();
    let row = |medium: &&str| serde_json::json!({ "medium": medium });
    let include = held.iter().map(row).collect::<Vec<_>>();
    let project = held
        .iter()
        .filter(|medium| **medium != "binary")
        .map(|medium| {
            prepared(medium)
                .map(|prepare| serde_json::json!({ "medium": medium, "prepare": prepare }))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let seal = held
        .iter()
        .filter(|medium| **medium == "binary")
        .map(|_| serde_json::json!({ "held": "seal" }))
        .collect::<Vec<_>>();
    Ok(serde_json::json!({
        "include": include,
        "project": { "include": project },
        "seal": { "include": seal },
    }))
}

fn prepared(medium: &str) -> Result<&'static str, String> {
    match medium {
        "cargo" | "cfworker" => Ok("rehearse"),
        "chart" => Ok("package"),
        "npm" => Ok("pack"),
        "oci" => Ok("build"),
        _ => Err(format!("{medium} names no deed that prepares it")),
    }
}
