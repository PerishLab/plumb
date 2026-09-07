use serde_json::Value;

pub fn workload(node: &Value) -> bool {
    node["reuse"]["type"] == "workload"
        && matches!(
            node["reason"].as_str(),
            Some("proof-held" | "publication-moved")
        )
}
