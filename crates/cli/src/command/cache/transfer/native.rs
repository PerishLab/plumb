const OVERRIDE: &str = "PLUMB_GUARD_NATIVE";
const FOLLOW: &str = "auto/";

pub(super) struct Skip {
    reason: &'static str,
    branch: Option<String>,
}

impl Skip {
    #[cfg(test)]
    pub fn reason(&self) -> &'static str {
        self.reason
    }

    pub fn report(&self) {
        crate::diagnostic::event(
            "material.skip",
            || serde_json::json!({"layer":"native","reason":self.reason,"branch":self.branch}),
        );
        let branch = self
            .branch
            .as_ref()
            .map(|name| format!(" {name}"))
            .unwrap_or_default();
        eprintln!(
            "guard material: native Cargo layer not packed ({}{branch}); saving compiler material only",
            self.reason
        );
    }
}

pub(super) fn skip() -> Result<Option<Skip>, String> {
    decide(|name| {
        if name == OVERRIDE {
            plumb::config::value(name)
        } else {
            std::env::var(name).ok()
        }
    })
}

pub(super) fn decide(read: impl Fn(&str) -> Option<String>) -> Result<Option<Skip>, String> {
    match read(OVERRIDE).as_deref() {
        None | Some("" | "auto") => {}
        Some("keep") => return Ok(None),
        Some("skip") => {
            return Ok(Some(Skip {
                reason: "requested",
                branch: None,
            }));
        }
        Some(other) => {
            return Err(format!(
                "{OVERRIDE} must be auto, keep or skip, not {other:?}"
            ));
        }
    }
    let branch = read("GITHUB_HEAD_REF")
        .filter(|name| !name.is_empty())
        .or_else(|| {
            read("GITHUB_REF").and_then(|name| name.strip_prefix("refs/heads/").map(str::to_string))
        });
    Ok(branch
        .filter(|name| name.starts_with(FOLLOW))
        .map(|name| Skip {
            reason: "follow-branch",
            branch: Some(name),
        }))
}
