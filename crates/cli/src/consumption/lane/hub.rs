use super::plan::Plan;
use std::process::Command;

pub(super) fn command() -> Command {
    let mut command = Command::new("gh");
    command.env_clear();
    for key in [
        "PATH",
        "HOME",
        "USERPROFILE",
        "SYSTEMROOT",
        "PATHEXT",
        "TMPDIR",
        "TEMP",
        "LANG",
        "XDG_CONFIG_HOME",
        "GH_CONFIG_DIR",
        "GH_TOKEN",
        "GITHUB_TOKEN",
        "SSL_CERT_FILE",
        "SSL_CERT_DIR",
    ] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    command
}

fn get(endpoint: &str, query: &str) -> Result<serde_json::Value, String> {
    let mut command = command();
    command.args([
        "api",
        "--hostname",
        "github.com",
        "--method",
        "GET",
        endpoint,
        "--jq",
        query,
    ]);
    let bytes = super::transport::capture(command, None)
        .map_err(|_| "GitHub observation refused or unavailable; no dispatch performed")?;
    serde_json::from_slice(&bytes).map_err(|_| "invalid GitHub lane observation".into())
}

pub(super) fn observe(plan: &Plan) -> Result<(), String> {
    let actor = get("user", "{login: .login}")?;
    let control = get("repos/PerishLab/wharf/commits/main", "{sha: .sha}")?;
    let workflow = format!(
        "PerishLab/wharf/.github/workflows/lane.yml@{}",
        control["sha"].as_str().ok_or("missing Wharf main commit")?
    );
    if actor["login"].as_str() != Some(plan.dispatch.caller())
        || workflow != plan.dispatch.workflow()
    {
        return Err("lane caller or fixed-main control moved; prepare and review again".into());
    }
    let source = get(
        &format!(
            "repos/{}/git/commits/{}",
            plan.source.repository(),
            plan.source.commit()
        ),
        "{sha: .sha, tree: .tree.sha}",
    )?;
    if source["sha"].as_str() != Some(plan.source.commit())
        || source["tree"].as_str() != Some(plan.source.tree())
    {
        return Err("lane source is not the exact pushed commit/tree".into());
    }
    Ok(())
}

pub(super) fn submit(plan: &Plan) -> Result<(), String> {
    let request =
        String::from_utf8(plan.dispatch.bytes()?).map_err(|_| "invalid lane dispatch encoding")?;
    let body = serde_json::to_vec(&serde_json::json!({"ref":"main","inputs":{"request":request}}))
        .map_err(|_| "cannot encode lane dispatch submission")?;
    let mut command = command();
    command.args([
        "api",
        "--hostname",
        "github.com",
        "--method",
        "POST",
        "repos/PerishLab/wharf/actions/workflows/lane.yml/dispatches",
        "--input",
        "-",
    ]);
    super::transport::capture(command, Some(&body))
        .map_err(|_| "Wharf dispatch acknowledgment unverified; operation outcome unknown; do not retry, replay or repair")?;
    Ok(())
}
