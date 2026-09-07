use serde::{Deserialize, Serialize};
use std::process::Output;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Probe {
    pub argv: Vec<String>,
    pub stdout: String,
    pub platform: Option<Vec<String>>,
}

impl Probe {
    pub fn select<'a>(rules: &'a [Self], platform: &str) -> Result<&'a Self, String> {
        for rule in rules {
            rule.validate()?;
        }
        let mut matching = rules.iter().filter(|rule| {
            rule.platform
                .as_ref()
                .is_none_or(|names| names.iter().any(|name| name == platform))
        });
        let selected = matching
            .next()
            .ok_or_else(|| format!("no probe rule for platform {platform}"))?;
        if matching.next().is_some() {
            return Err(format!("conflicting probe rules for platform {platform}"));
        }
        Ok(selected)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.argv.first().is_none_or(|program| program.is_empty())
            || self.argv.iter().any(|arg| arg.contains('\0'))
        {
            return Err("probe requires a nonempty program and valid argv".into());
        }
        if self
            .platform
            .as_ref()
            .is_some_and(|names| names.is_empty() || names.iter().any(|name| name.is_empty()))
        {
            return Err("probe platform must name at least one platform".into());
        }
        Ok(())
    }

    pub fn check(&self, output: &Output) -> Result<bool, String> {
        self.validate()?;
        if !output.status.success() {
            return Err(format!(
                "probe {} failed with {}",
                self.argv[0], output.status
            ));
        }
        let actual =
            std::str::from_utf8(&output.stdout).map_err(|_| "probe stdout is not UTF-8")?;
        Ok(self.accepts(actual))
    }

    pub fn accepts(&self, actual: &str) -> bool {
        normalized(actual) == normalized(&self.stdout)
    }
}

fn normalized(text: &str) -> String {
    text.replace("\r\n", "\n")
        .trim_end_matches('\n')
        .to_string()
}
