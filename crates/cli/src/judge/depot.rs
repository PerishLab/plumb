use super::finding::{Finding, Seed};
use crate::catalog::rules::config::PLUMB_CURRENT;
use std::process::Command;

const MARK: &str = "PLUMB_DEPOT_MARK";

pub fn mark() -> Vec<Finding> {
    let Some(raw) = std::env::var_os(MARK) else {
        return Vec::new();
    };
    let Ok(expected) = raw.into_string() else {
        return vec![Finding::new(Seed::blind(
            &PLUMB_CURRENT,
            format!("{MARK} is not valid UTF-8"),
        ))];
    };
    if !valid(&expected) {
        return vec![Finding::new(Seed::blind(
            &PLUMB_CURRENT,
            format!("{MARK} must be a 64-character lowercase hex depot mark"),
        ))];
    }
    let active = match plumb::depot::rules() {
        Ok(rules) => rules.mark(),
        Err(error) => {
            return vec![Finding::new(Seed::blind(
                &PLUMB_CURRENT,
                format!("cannot read active Plumb depot mark: {error}"),
            ))];
        }
    };
    if expected == active {
        Vec::new()
    } else {
        vec![Finding::new(Seed::wrong(
            &PLUMB_CURRENT,
            format!("pinned depot mark {expected} disagrees with active Plumb depot {active}"),
        ))]
    }
}

fn valid(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub fn runtime() -> Vec<Finding> {
    let Some(manager) = managed() else {
        return Vec::new();
    };
    let running = plumb::version!("PLUMB");
    let state = match std::fs::read_to_string(&manager) {
        Ok(state) => state,
        Err(error) => {
            return vec![Finding::new(Seed::blind(
                &PLUMB_CURRENT,
                format!(
                    "cannot read managed Plumb identity {}: {error}",
                    manager.display()
                ),
            ))];
        }
    };
    let field = |name: &str| {
        state
            .lines()
            .find_map(|line| line.split_once('=').filter(|(key, _)| *key == name))
            .map(|(_, value)| value.trim())
    };
    if field("channel") != Some("stable") || field("version") != Some(running) {
        return vec![Finding::new(Seed::wrong(
            &PLUMB_CURRENT,
            format!("managed Plumb identity is not stable {running}; reinstall stable latest"),
        ))];
    }
    let releases = authority();
    let latest = match latest(&releases) {
        Ok(latest) => latest,
        Err(finding) => return vec![finding],
    };
    if latest == running {
        Vec::new()
    } else {
        vec![Finding::new(Seed::wrong(
            &PLUMB_CURRENT,
            format!(
                "the running Plumb is {running}, stable latest is {latest}; reinstall stable latest",
            ),
        ))]
    }
}

pub(crate) struct Control {
    running: String,
    releases: String,
}

impl Control {
    pub(crate) fn snapshot() -> Option<Self> {
        let manager = managed()?;
        let running = plumb::version!("PLUMB");
        let state = std::fs::read_to_string(&manager).ok()?;
        let field = |name: &str| {
            state
                .lines()
                .find_map(|line| line.split_once('=').filter(|(key, _)| *key == name))
                .map(|(_, value)| value.trim())
        };
        if field("channel") != Some("stable") || field("version") != Some(running) {
            return None;
        }
        let releases = authority();
        let started = latest(&releases).ok()?;
        matched(running, &started).then(|| Self {
            running: running.to_string(),
            releases,
        })
    }

    pub(crate) fn rolled(
        &self,
        root: &std::path::Path,
        execution: Option<&plumb::config::Execution>,
    ) -> Option<String> {
        let current = latest(&self.releases).ok()?;
        if current == self.running {
            return None;
        }
        let mut command = match execution {
            Some(execution) => execution.command("plumb").ok()?,
            None => Command::new(plumb::config::binary()?),
        };
        command.current_dir(root).args(["doctor", ".", "--json"]);
        let output = command.output().ok()?;
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
        pure(&report).then(|| {
            format!(
                "guard.control.rollover: running Plumb {} rolled over to stable {current} during Guard",
                self.running
            )
        })
    }
}

pub(crate) fn matched(running: &str, latest: &str) -> bool {
    running == latest
}

pub(crate) fn pure(report: &serde_json::Value) -> bool {
    let Some(findings) = report.get("findings").and_then(serde_json::Value::as_array) else {
        return false;
    };
    let mut current = false;
    for finding in findings {
        let grade = finding
            .get("grade")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        if !matches!(grade, "out of true" | "blind") {
            continue;
        }
        let code = finding
            .get("code")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        if code != PLUMB_CURRENT.0 {
            return false;
        }
        current = true;
    }
    current
}

fn authority() -> String {
    plumb::rig::Rig::resolve(None, &plumb::rig::Rig::prefix(), &plumb::config::value)
        .map(|rig| rig.releases)
        .unwrap_or_else(|_| "https://releases.plumb.perish.uk".to_string())
}

fn latest(releases: &str) -> Result<String, Finding> {
    let url = plumb::seat::release::Authority::at(releases).pointer("stable");
    let output = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--connect-timeout",
            "5",
            "--max-time",
            "15",
        ])
        .arg(&url)
        .output();
    let output = match output {
        Ok(output) if output.status.success() => output,
        Ok(output) => {
            return Err(Finding::new(Seed::blind(
                &PLUMB_CURRENT,
                format!(
                    "cannot read Plumb stable latest at {url}: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ),
            )));
        }
        Err(error) => {
            return Err(Finding::new(Seed::blind(
                &PLUMB_CURRENT,
                format!("cannot read Plumb stable latest at {url}: {error}"),
            )));
        }
    };
    let pointer: serde_json::Value = serde_json::from_slice(&output.stdout).map_err(|error| {
        Finding::new(Seed::blind(
            &PLUMB_CURRENT,
            format!("cannot parse Plumb stable latest at {url}: {error}"),
        ))
    })?;
    let version = pointer
        .get("releaseVersion")
        .and_then(serde_json::Value::as_str);
    let valid = pointer.get("schema").and_then(serde_json::Value::as_u64) == Some(1)
        && pointer.get("product").and_then(serde_json::Value::as_str) == Some("plumb")
        && pointer.get("channel").and_then(serde_json::Value::as_str) == Some("stable");
    match version {
        Some(version) if valid => Ok(version.to_string()),
        _ => Err(Finding::new(Seed::blind(
            &PLUMB_CURRENT,
            format!("Plumb stable latest at {url} has an unknown shape"),
        ))),
    }
}

fn managed() -> Option<std::path::PathBuf> {
    let executable = plumb::config::binary()?;
    let marker = executable.parent()?.join(".plumb-manager");
    marker.is_file().then_some(marker)
}

#[cfg(test)]
mod tests {
    use super::{matched, pure};
    use serde_json::json;

    #[test]
    fn rollover() {
        assert!(matched("v0.166.0", "v0.166.0"));
        assert!(!matched("v0.166.0", "v0.167.0"));

        let solely = json!({
            "findings": [
                {"code": "config.plumb-current-and-guarded", "grade": "out of true"},
                {"code": "deps.pin", "grade": "noted"}
            ]
        });
        assert!(pure(&solely));

        let mixed = json!({
            "findings": [
                {"code": "config.plumb-current-and-guarded", "grade": "out of true"},
                {"code": "structure.layout", "grade": "out of true"}
            ]
        });
        assert!(!pure(&mixed));

        let real = json!({
            "findings": [
                {"code": "structure.layout", "grade": "out of true"}
            ]
        });
        assert!(!pure(&real));
        assert!(!pure(&json!({"findings": []})));
    }
}
