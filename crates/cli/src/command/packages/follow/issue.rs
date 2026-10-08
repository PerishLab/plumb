use super::provider::Provider;
use serde_json::{Value, json};

const MARKER: &str = "<!-- plumb.auto/v1 operation=follow -->";

pub(super) fn find(provider: &Provider<'_>) -> Result<Option<u64>, String> {
    let issues = provider.pages(&format!(
        "repos/{}/issues?state=open&per_page=100",
        provider.repository
    ))?;
    let matching = issues
        .iter()
        .filter(|issue| {
            issue["body"]
                .as_str()
                .is_some_and(|body| body.contains(MARKER))
                && issue.get("pull_request").is_none()
        })
        .collect::<Vec<_>>();
    if matching.len() > 1 {
        return Err("multiple open Auto follow issues require human judgment".into());
    }
    matching
        .first()
        .map(|issue| {
            if issue["type"]["name"] != "Auto" {
                return Err("matching follow issue is not Auto; human judgment required".into());
            }
            issue["number"]
                .as_u64()
                .ok_or_else(|| "Auto issue has no number".into())
        })
        .transpose()
}

pub(super) fn create(provider: &Provider<'_>) -> Result<u64, String> {
    let issue = provider.api(&format!("repos/{}/issues", provider.repository), Some(&json!({"title":"Follow the latest first-party packages", "type":"Auto", "body":body(provider.repository)})))?;
    issue["number"]
        .as_u64()
        .ok_or_else(|| "created Auto issue has no number; reread before retry".into())
}

pub(super) fn body(repository: &str) -> String {
    format!(
        "## Operation\nfollow\n\n## Target\n{repository}\n\n## Change\nSet first-party requirements to 0 and resolve crates from the perish registry and @perishlab npm packages to their latest stable. Allowed paths: Cargo.toml, package.json, Cargo.lock and pnpm-lock.yaml at the root or in repository packages. The pull request records the exact delta.\n\n## Acceptance\n- [ ] The bounded change is merged through the required organization Guard.\n- [ ] Closure records the read-back merge commit and successful Guard run for the delivered change.\n\n## Non-goals\nBusiness-source repair, unrelated third-party upgrades, releases and organization settings.\n\n{MARKER}\n"
    )
}

pub(super) fn snapshot(
    provider: &Provider<'_>,
    number: u64,
) -> Result<plumb::delivery::Snapshot, String> {
    let issue = provider.issue(number)?;
    if issue["type"]["name"] != "Auto"
        || !issue["body"].as_str().is_some_and(|body| {
            body.contains(MARKER) && body.contains(&format!("## Target\n{}\n", provider.repository))
        })
    {
        return Err("Issue is not this repository's registered Auto follow".into());
    }
    if issue["body"].as_str() != Some(body(provider.repository).as_str())
        && issue["body"].as_str()
            != Some(body(provider.repository).replace("- [ ]", "- [x]").as_str())
    {
        return Err("Auto follow body changed outside its operation contract".into());
    }
    let children = provider.pages(&format!(
        "repos/{}/issues/{number}/sub_issues?per_page=100",
        provider.repository
    ))?;
    if children.iter().any(|issue| issue["state"] != "closed") {
        return Err("Auto issue has an unresolved child".into());
    }
    for label in issue["labels"]
        .as_array()
        .ok_or("Issue labels are unreadable")?
    {
        if label["name"]
            .as_str()
            .is_some_and(|name| name.starts_with("needs:") || name.starts_with("acceptance:"))
        {
            return Err("Auto issue needs human resolution of its labels".into());
        }
    }
    let blockers = provider.pages(&format!(
        "repos/{}/issues/{number}/dependencies/blocked_by?per_page=100",
        provider.repository
    ))?;
    if blockers.iter().any(|issue| issue["state"] != "closed") {
        return Err("Auto issue has an unresolved native blocker".into());
    }
    Ok(plumb::delivery::Snapshot {
        node: text(&issue, "node_id")?,
        repository: provider.repository.into(),
        number,
        url: text(&issue, "html_url")?,
        title: text(&issue, "title")?,
        state: text(&issue, "state")?.to_uppercase(),
        kind: "Auto".into(),
        updated: text(&issue, "updated_at")?,
        parent: None,
        sub_issues: Vec::new(),
        blocked_by: Vec::new(),
        blocking: Vec::new(),
    })
}

pub(super) fn text(value: &Value, key: &str) -> Result<String, String> {
    value[key]
        .as_str()
        .filter(|text| !text.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("provider field {key} is missing"))
}
