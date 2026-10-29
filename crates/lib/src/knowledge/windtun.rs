use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::Path;

pub const SCHEMA: &str = "plumb.windtun.declaration/v1";

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Inspection {
    pub schema: &'static str,
    pub manifest: Option<String>,
    pub windtun: Option<Declaration>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Declaration {
    pub scenes: Scenes,
    pub cases: Vec<String>,
    pub image: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Scenes {
    pub authority: String,
    pub product: String,
    pub channel: String,
}

pub fn parse(text: &str) -> Result<Option<Declaration>, String> {
    let document: toml::Table = text
        .parse()
        .map_err(|error| format!("invalid plumb.toml: {error}"))?;
    let legacy = match document.get("guard") {
        None => None,
        Some(value) => {
            let guard = value.as_table().ok_or("guard must be a table")?;
            if guard.keys().any(|key| key != "seatrial") {
                return Err("guard declares only the legacy Seatrial suite".into());
            }
            guard.get("seatrial")
        }
    };
    let value = match (document.get("windtun"), legacy) {
        (None, None) => return Ok(None),
        (Some(_), Some(_)) => {
            return Err("declare windtun once; remove legacy guard.seatrial".into());
        }
        (Some(value), None) | (None, Some(value)) => value,
    };
    let held: Declaration = value
        .clone()
        .try_into()
        .map_err(|error| format!("windtun: {error}"))?;
    held.validate()?;
    Ok(Some(held))
}

pub fn inspect(root: &Path) -> Result<Inspection, String> {
    if !root.is_dir() {
        return Err("Windtun inspection requires an existing repository directory".into());
    }
    let path = root.join("plumb.toml");
    let metadata = match std::fs::symlink_metadata(&path) {
        Ok(metadata) => Some(metadata),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(format!("cannot inspect plumb.toml: {error}")),
    };
    let Some(metadata) = metadata else {
        return Ok(Inspection {
            schema: SCHEMA,
            manifest: None,
            windtun: None,
        });
    };
    if !metadata.file_type().is_file() {
        return Err("Windtun declaration requires a regular plumb.toml".into());
    }
    let raw = std::fs::read(path).map_err(|error| format!("cannot read plumb.toml: {error}"))?;
    let text = std::str::from_utf8(&raw).map_err(|_| "invalid declaration UTF-8")?;
    Ok(Inspection {
        schema: SCHEMA,
        manifest: Some(format!("{:x}", Sha256::digest(&raw))),
        windtun: parse(text)?,
    })
}

pub fn declared(snapshot: &crate::snapshot::Snapshot) -> Result<Option<Declaration>, String> {
    let Some(entry) = snapshot
        .entries()
        .iter()
        .find(|entry| entry.path() == "plumb.toml")
    else {
        return Ok(None);
    };
    if !matches!(entry.mode(), "100644" | "100755") {
        return Err("Seatrial declaration requires a tracked regular plumb.toml".into());
    }
    parse(std::str::from_utf8(entry.bytes()).map_err(|_| "invalid declaration UTF-8")?)
}

impl Declaration {
    pub fn validate(&self) -> Result<(), String> {
        let scenes = &self.scenes;
        let authority = &scenes.authority;
        crate::seat::release::Authority::new(authority)?;
        if authority.contains(['?', '#', '@']) {
            return Err("Seatrial scenes require a normalized public HTTPS authority".into());
        }
        if !name(&scenes.product)
            || !["stable", "alpha", "beta", "rc"].contains(&scenes.channel.as_str())
        {
            return Err(
                "Seatrial scenes require a product and channel, never a version pin".into(),
            );
        }
        let unique: BTreeSet<_> = self.cases.iter().collect();
        if self.cases.is_empty()
            || unique.len() != self.cases.len()
            || self.cases.iter().any(|case| !name(case))
        {
            return Err("Seatrial cases must be explicit nonempty unique names".into());
        }
        if self.image.is_empty() || self.image.chars().any(char::is_whitespace) {
            return Err("Seatrial requires a runtime image reference".into());
        }
        if self.image.starts_with("ghcr.io/perishlab/") {
            let tag = self.image.rsplit('/').next().unwrap_or_default();
            if self.image.contains('@')
                || tag
                    .split_once(':')
                    .is_some_and(|(_, tag)| !["stable", "alpha", "beta", "rc"].contains(&tag))
            {
                return Err(
                    "first-party Seatrial images follow a channel or the resolved product release"
                        .into(),
                );
            }
        } else if !self
            .image
            .rsplit_once("@sha256:")
            .is_some_and(|(_, digest)| {
                digest.len() == 64
                    && digest
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            })
        {
            return Err("third-party Seatrial images require an immutable SHA-256 digest".into());
        }
        Ok(())
    }
}

fn name(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DECLARED: &str = r#"[guard.seatrial]
cases = ["health-lite", "again-lite"]
image = "ghcr.io/perishlab/keel"
[guard.seatrial.scenes]
authority = "https://depot.seatrial.perish.uk"
product = "seatrial"
channel = "beta"
"#;

    #[test]
    fn products() {
        let keel = parse(DECLARED).unwrap().unwrap();
        assert_eq!(keel.cases, ["health-lite", "again-lite"]);
        let plumb = DECLARED.replace("[\"health-lite\", \"again-lite\"]", "[\"writer\"]");
        let plumb = plumb.replace("ghcr.io/perishlab/keel", "ghcr.io/perishlab/images:stable");
        assert!(parse(&plumb).unwrap().is_some());
        assert_eq!(parse("[identity]\nns = \"keel\"\n").unwrap(), None);
        let modern = DECLARED.replace("guard.seatrial", "windtun");
        assert_eq!(parse(&modern).unwrap(), parse(DECLARED).unwrap());
        assert!(parse(&(DECLARED.to_owned() + &modern)).is_err());
    }

    #[test]
    fn refuse() {
        for (from, to) in [
            ("channel = \"beta\"", "channel = \"v1.2.3-beta.1\""),
            (
                "https://depot.seatrial.perish.uk",
                "http://depot.seatrial.perish.uk",
            ),
            ("[\"health-lite\", \"again-lite\"]", "[]"),
            ("again-lite", "health-lite"),
            ("health-lite", "*"),
            ("ghcr.io/perishlab/keel", "ghcr.io/perishlab/keel:0.27.0"),
            ("image =", "command ="),
        ] {
            assert!(
                parse(&DECLARED.replace(from, to)).is_err(),
                "{from} -> {to}"
            );
        }
        assert!(parse(&(DECLARED.to_owned() + "version = \"v1.2.3\"\n")).is_err());
        let modern = DECLARED.replace("guard.seatrial", "windtun");
        assert!(parse(&modern.replace("again-lite", "health-lite")).is_err());
        assert!(parse(&modern.replace("channel = \"beta\"", "channel = \"v1.2.3\"")).is_err());
        assert!(parse(&modern.replace("image =", "command =")).is_err());
    }
}
