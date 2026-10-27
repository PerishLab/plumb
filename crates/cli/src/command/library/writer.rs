use serde_json::Value;

pub fn check(
    repository: &str,
    issue: u64,
    command: impl Fn(&[String]) -> Result<Vec<u8>, String>,
) -> Result<(), String> {
    let bytes = command(&["api".into(), format!("repos/{repository}/issues/{issue}")])?;
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("provider returned invalid JSON: {error}"))?;
    let (owner, writer) =
        actor(&value["user"], "id").ok_or("Auto issue has no operation writer identity")?;
    let bytes = command(
        &[
            "api",
            "graphql",
            "-f",
            "query=query{viewer{databaseId login}}",
        ]
        .map(String::from),
    )?;
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("provider identity is unreadable: {error}"))?;
    let (id, operator) =
        actor(&value["data"]["viewer"], "databaseId").ok_or("provider identity is unreadable")?;
    if id != owner {
        return Err(format!(
            "Auto #{issue} has operation writer {writer} (id {owner}), but this follow runs as {operator} (id {id}); only the writer settles its automation-owned Auto, so nothing was written; rerun follow as {writer}"
        ));
    }
    Ok(())
}

fn actor(value: &Value, key: &str) -> Option<(u64, String)> {
    let login = value["login"].as_str().filter(|login| !login.is_empty())?;
    let id = value[key].as_u64().filter(|id| *id > 0)?;
    Some((id, login.to_string()))
}
