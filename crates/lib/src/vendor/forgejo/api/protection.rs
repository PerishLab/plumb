use super::{Client, failure, segment};
use serde_json::{Value, json};

impl Client {
    pub fn protect(&self, name: &str, mode: &str) -> Result<(), String> {
        let username = self.user()?;
        let route = format!("/branch_protections/{}", segment(name));
        let existing = self.request("GET", &route, None)?;
        if ![200, 404].contains(&existing.status) {
            return Err(failure(
                "fetching branch protection",
                existing.status,
                &existing.value,
            ));
        }
        let policy = policy(name, mode, &username);
        let response = if existing.status == 404 {
            self.request("POST", "/branch_protections", Some(policy.clone()))?
        } else {
            self.request("PATCH", &route, Some(policy.clone()))?
        };
        let wanted = if existing.status == 404 { 201 } else { 200 };
        if response.status != wanted {
            return Err(failure(
                "setting branch protection",
                response.status,
                &response.value,
            ));
        }
        verify(name, &policy, &response.value)
    }
}

pub fn policy(name: &str, mode: &str, username: &str) -> Value {
    let preparing = mode == "preparing";
    json!({
        "rule_name": name,
        "enable_push": preparing,
        "enable_push_whitelist": preparing,
        "push_whitelist_usernames": if preparing { vec![username] } else { Vec::new() },
        "push_whitelist_teams": [],
        "push_whitelist_deploy_keys": false,
        "enable_merge_whitelist": false,
        "merge_whitelist_usernames": [],
        "merge_whitelist_teams": [],
        "enable_status_check": false,
        "status_check_contexts": Value::Null,
        "required_approvals": 0,
        "enable_approvals_whitelist": false,
        "approvals_whitelist_username": [],
        "approvals_whitelist_teams": [],
        "block_on_rejected_reviews": false,
        "block_on_official_review_requests": false,
        "block_on_outdated_branch": false,
        "dismiss_stale_approvals": false,
        "ignore_stale_approvals": false,
        "require_signed_commits": false,
        "protected_file_patterns": "",
        "unprotected_file_patterns": "",
        "apply_to_admins": true
    })
}

pub fn verify(name: &str, expected: &Value, actual: &Value) -> Result<(), String> {
    if actual.get("branch_name").and_then(Value::as_str) != Some(name)
        || expected.as_object().is_none_or(|fields| {
            fields
                .iter()
                .any(|(key, value)| actual.get(key) != Some(value))
        })
    {
        Err("branch protection did not read back byte-for-byte".into())
    } else {
        Ok(())
    }
}
