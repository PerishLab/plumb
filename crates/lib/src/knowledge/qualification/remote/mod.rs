mod material;
mod metadata;

use super::{Fault, Read, Result, registry::Registry, wire::ARCHIVE};
use material::Material;
use std::sync::OnceLock;
use std::time::Duration;

pub struct Remote {
    agent: ureq::Agent,
    github: OnceLock<std::result::Result<runseal::github::Client, String>>,
}

impl Default for Remote {
    fn default() -> Self {
        Self {
            github: OnceLock::new(),
            agent: ureq::Agent::config_builder()
                .https_only(true)
                .max_redirects(0)
                .timeout_global(Some(Duration::from_secs(10)))
                .build()
                .into(),
        }
    }
}

impl Read for Remote {
    fn bytes(&self, url: &str, limit: usize) -> Result<Vec<u8>> {
        if limit > ARCHIVE {
            return Err(Fault::refused("public read budget exceeds protocol bounds"));
        }
        if let Some(path) = url.strip_prefix("https://api.github.com/") {
            let client = self
                .github
                .get_or_init(crate::config::github::client)
                .as_ref()
                .map_err(|error| Fault::unknown(error.clone()))?;
            return metadata::read(client, path, limit);
        }
        Material::new(url).read(&self.agent, limit)
    }

    fn image(&self, reference: &str) -> Result<String> {
        Registry(&self.agent).inspect(reference)
    }

    fn optional(&self, url: &str, limit: usize) -> Result<Option<Vec<u8>>> {
        if limit > ARCHIVE {
            return Err(Fault::refused(
                "public optional read exceeds protocol bounds",
            ));
        }
        if url.starts_with("https://api.github.com/") {
            return Err(Fault::refused("optional native metadata is unsupported"));
        }
        Material::new(url).optional(&self.agent, limit)
    }
}
