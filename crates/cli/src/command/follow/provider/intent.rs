use super::Provider;
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(in super::super) struct Intent {
    pub schema: String,
    pub repository: String,
    pub issue: u64,
    pub branch: String,
    pub previous: Option<String>,
    pub head: String,
    pub base: String,
    pub tree: String,
}

impl Intent {
    fn check(&self, provider: &Provider<'_>, issue: u64) -> Result<(), String> {
        let exact = |head: &str| {
            matches!(head.len(), 40 | 64) && head.bytes().all(|byte| byte.is_ascii_hexdigit())
        };
        let identity = (self.schema.as_str(), self.repository.as_str(), self.issue);
        if identity != ("plumb.auto-push/v1", provider.repository, issue)
            || self.branch != format!("auto/{issue}")
        {
            return Err("Auto publication intent identity disagrees".into());
        }
        if ![&self.head, &self.base, &self.tree]
            .into_iter()
            .all(|head| exact(head))
            || self.previous.as_deref().is_some_and(|head| !exact(head))
        {
            return Err("Auto publication intent has an invalid Git identity".into());
        }
        Ok(())
    }

    fn body(&self) -> Result<String, String> {
        Ok(format!(
            "Auto follow publication intent; provider readback decides execution.\n\n<!-- plumb.auto-push/v1\n{}\n-->",
            serde_json::to_string(self).map_err(|error| error.to_string())?
        ))
    }
}

impl Provider<'_> {
    pub(in super::super) fn intent(&self, issue: u64, held: &Intent) -> Result<(), String> {
        held.check(self, issue)?;
        self.comment(issue, &held.body()?)?;
        if self.recorded(issue)?.as_ref() != Some(held) {
            return Err("Auto publication intent readback disagrees".into());
        }
        Ok(())
    }

    pub(in super::super) fn recorded(&self, issue: u64) -> Result<Option<Intent>, String> {
        let mut standing = None;
        for body in self.evidence(issue, "<!-- plumb.auto-push/", "Auto publication intent")? {
            let value = body
                .split_once("<!-- plumb.auto-push/v1\n")
                .and_then(|(_, value)| value.strip_suffix("\n-->"))
                .ok_or("Auto publication intent has an unknown shape")?;
            let held: Intent = serde_json::from_str(value)
                .map_err(|error| format!("Auto publication intent is unreadable: {error}"))?;
            held.check(self, issue)?;
            standing = Some(held);
        }
        Ok(standing)
    }
}
