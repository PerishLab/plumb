use super::{Factory, Resource, detail, field};
use serde_json::{Value, json};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Policy {
    before: Value,
    resources: Value,
}

impl Policy {
    pub fn read(before: Value, resources: Value, permission: &str) -> Result<Self, String> {
        if before["status"] != "active" {
            return Err("ship writer is not active".into());
        }
        let policies = before["policies"]
            .as_array()
            .ok_or("ship writer has no policies")?;
        if policies.len() != 1 || policies[0]["effect"] != "allow" {
            return Err("ship writer must carry one allow policy".into());
        }
        let grants = policies[0]["permission_groups"]
            .as_array()
            .ok_or("ship writer has no permission groups")?;
        if grants.len() != 1 || grants[0]["id"] != permission {
            return Err("ship writer has an unexpected permission group".into());
        }
        if !policies[0]["resources"].is_object() {
            return Err("ship writer resources are unreadable".into());
        }
        Ok(Self { before, resources })
    }

    pub fn pending(&self) -> bool {
        self.before["policies"][0]["resources"] != self.resources
    }

    pub fn detail(&self) -> String {
        format!(
            "converge writer bucket resources from {} to {} without rotating its identity",
            self.before["policies"][0]["resources"], self.resources
        )
    }

    fn body(&self) -> Value {
        let mut body = json!({
            "name": self.before["name"],
            "status": self.before["status"],
            "policies": self.before["policies"],
        });
        body["policies"][0]["resources"] = self.resources.clone();
        for key in ["condition", "expires_on", "not_before"] {
            if let Some(value) = self.before.get(key) {
                body[key] = value.clone();
            }
        }
        body
    }
}

impl Factory {
    pub fn policy(&self, id: &str, buckets: &[String]) -> Result<Policy, String> {
        let before = self
            .call(&["token", "account", "show", id], None)
            .map_err(detail)?;
        if field(&before.value, "id")? != id
            || field(&before.value, "name")? != "ship:release-buckets"
        {
            return Err("ship writer identity changed".into());
        }
        let resources = Resource::Set {
            account: self.id().to_string(),
            buckets: buckets.to_vec(),
        }
        .policy();
        let permission = self.permission(
            "Workers R2 Storage Bucket Item Write",
            "com.cloudflare.edge.r2.bucket",
        )?;
        Policy::read(before.value, resources, &permission)
    }

    pub fn reconcile(&self, id: &str, buckets: &[String]) -> Result<(), String> {
        let planned = self.policy(id, buckets)?;
        if !planned.pending() {
            return Ok(());
        }
        let fresh = self.policy(id, buckets)?;
        if fresh != planned {
            return Err("ship writer policy changed before update".into());
        }
        self.call(&["token", "account", "edit", id], Some(planned.body()))
            .map_err(detail)?;
        if self.policy(id, buckets)?.pending() {
            return Err("ship writer policy update did not verify".into());
        }
        Ok(())
    }
}
