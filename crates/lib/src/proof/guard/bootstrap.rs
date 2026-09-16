use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Bootstrap {
    pub marker: crate::depot::v3::Marker,
    pub generation: String,
    pub controller: String,
    pub configuration: String,
    pub validator: super::Validator,
}

impl Bootstrap {
    pub fn validate(&self) -> Result<(), String> {
        for digest in [
            &self.marker.sha256,
            &self.generation,
            &self.controller,
            &self.configuration,
            &self.validator.release,
            &self.validator.artifact,
        ] {
            super::hash(digest, "bootstrap")?;
            if digest.len() != 64 {
                return Err("bootstrap evidence requires SHA256 digests".into());
            }
        }
        semver::Version::parse(self.marker.name.trim_start_matches('v'))
            .map_err(|error| format!("invalid bootstrap marker: {error}"))?;
        if self.validator.version != self.marker.name {
            return Err("bootstrap validator must match its explicit marker".into());
        }
        Ok(())
    }

    pub fn executable() -> Result<String, String> {
        static DIGEST: std::sync::OnceLock<Result<String, String>> = std::sync::OnceLock::new();
        DIGEST.get_or_init(Self::measure).clone()
    }

    fn measure() -> Result<String, String> {
        use sha2::{Digest as _, Sha256};
        let path = crate::config::binary().ok_or("cannot locate controller executable")?;
        let mut file = std::fs::File::open(path).map_err(|error| error.to_string())?;
        let mut hash = Sha256::new();
        std::io::copy(&mut file, &mut hash).map_err(|error| error.to_string())?;
        Ok(format!("{:x}", hash.finalize()))
    }

    pub fn current(&self) -> Result<(), String> {
        self.validate()?;
        if self.controller != Self::executable()? {
            return Err("bootstrap proof belongs to another controller executable".into());
        }
        Ok(())
    }
}
