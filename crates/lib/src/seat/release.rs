use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Authority {
    base: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Marker {
    pub channel: String,
    pub marker: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Distribution {
    pub marker: String,
    pub commit: String,
    pub state: String,
    pub media: BTreeMap<String, String>,
    pub run: Option<String>,
}

#[derive(Deserialize)]
struct Manifest {
    release: Option<Section>,
}

#[derive(Deserialize)]
struct Section {
    #[serde(default)]
    authority: String,
}

const PUBLIC: [&str; 5] = ["published", "deployed", "pointed", "present", "skipped"];

impl Authority {
    pub fn new(base: &str) -> Result<Self, String> {
        if !base.starts_with("https://")
            || base.ends_with('/')
            || base.chars().any(char::is_whitespace)
        {
            return Err("authority must be one normalized https URL".into());
        }
        Ok(Self {
            base: base.to_string(),
        })
    }

    pub fn declared(manifest: &str) -> Result<Option<Self>, String> {
        let held: Manifest = toml::from_str(manifest)
            .map_err(|error| format!("cannot parse plumb.toml: {error}"))?;
        match held.release {
            Some(section) if !section.authority.is_empty() => {
                Self::new(&section.authority).map(Some)
            }
            _ => Ok(None),
        }
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    pub fn pointer(&self, channel: &str) -> String {
        format!("{}/v1/channels/{channel}.json", self.base)
    }

    pub fn seal(&self, channel: &str, marker: &str) -> String {
        format!("{}/v1/releases/{channel}/{marker}/seal.json", self.base)
    }

    pub fn distribution(&self, channel: &str, marker: &str) -> String {
        format!(
            "{}/v1/releases/{channel}/{marker}/distribution.json",
            self.base
        )
    }

    pub fn record(&self, reference: &str) -> Option<Marker> {
        let path = reference
            .strip_prefix(self.base.as_str())?
            .strip_prefix("/v1/releases/")?
            .strip_suffix("/distribution.json")?;
        let (channel, marker) = path.split_once('/')?;
        let segment = |part: &str| !part.is_empty() && !part.contains('/');
        (segment(channel) && segment(marker)).then(|| Marker {
            channel: channel.to_string(),
            marker: marker.to_string(),
        })
    }

    #[cfg(feature = "depot")]
    pub fn read(&self, marker: &Marker) -> Result<Option<Distribution>, String> {
        let url = self.distribution(&marker.channel, &marker.marker);
        super::bucket::fetch(&url)
            .map_err(|error| format!("cannot read {url}: {error}"))?
            .map(|body| Distribution::parse(&body, &marker.marker))
            .transpose()
    }
}

impl Distribution {
    pub fn parse(body: &[u8], marker: &str) -> Result<Self, String> {
        let held: Value = serde_json::from_slice(body)
            .map_err(|error| format!("{marker} distribution record does not parse: {error}"))?;
        if held["marker"] != marker {
            return Err(format!(
                "{marker} distribution record names {}",
                held["marker"]
            ));
        }
        let text = |value: &Value| value.as_str().unwrap_or_default().to_string();
        let media = held["media"]
            .as_object()
            .map(|media| {
                media
                    .iter()
                    .map(|(name, state)| (name.clone(), text(state)))
                    .collect()
            })
            .unwrap_or_default();
        Ok(Self {
            marker: marker.to_string(),
            commit: text(&held["commit"]),
            state: text(&held["state"]),
            media,
            run: held["attempt"]["run"].as_str().map(str::to_string),
        })
    }

    pub fn complete(&self) -> bool {
        self.state == "complete"
    }

    pub fn public(&self) -> Vec<String> {
        self.media
            .iter()
            .filter(|(_, state)| PUBLIC.contains(&state.as_str()))
            .map(|(name, _)| name.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests;
