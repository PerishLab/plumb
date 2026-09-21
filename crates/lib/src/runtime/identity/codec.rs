use super::region::{MAGIC, PAYLOAD, SIZE};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub product: String,
    pub marker: String,
    pub digest: String,
    pub commit: String,
    pub workload: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Origin {
    pub prefix: String,
    pub commit: String,
    pub target: String,
}

impl Binding {
    pub fn channel(&self) -> Result<&str, String> {
        let raw = self
            .marker
            .strip_prefix('v')
            .ok_or("identity marker requires v")?;
        let version = semver::Version::parse(raw).map_err(|error| error.to_string())?;
        if !version.build.is_empty() {
            return Err("identity marker cannot carry build metadata".into());
        }
        match self.marker.split_once('-') {
            None => Ok("stable"),
            Some((_, pre)) => match pre.split_once('.') {
                Some((channel @ ("alpha" | "beta" | "rc"), number))
                    if number.parse::<u64>().is_ok_and(|value| value > 0) =>
                {
                    Ok(channel)
                }
                _ => Err("identity marker carries an unsupported channel".into()),
            },
        }
    }

    pub(super) fn verify(&self, origin: &Origin) -> Result<(), String> {
        self.channel()?;
        if self.product.is_empty()
            || self.product.to_ascii_uppercase().replace('-', "_") != origin.prefix
        {
            return Err("identity product differs from the executable".into());
        }
        hex(&self.digest, 64)?;
        hex(&self.commit, 40)?;
        hex(&self.workload, 64)?;
        Ok(())
    }
}

pub struct Codec<'a>(pub &'a [u8]);
impl Codec<'_> {
    pub fn decode(&self) -> Result<(Origin, Option<Binding>), String> {
        let bytes = self.0;
        if bytes.len() != SIZE || &bytes[..16] != MAGIC {
            return Err("identity region has an unknown format or size".into());
        }
        let origin = Origin {
            prefix: text(&bytes[16..80])?,
            commit: text(&bytes[80..120])?,
            target: text(&bytes[120..248])?,
        };
        if origin.prefix.is_empty()
            || !origin
                .prefix
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b == b'_')
        {
            return Err("identity region carries an invalid product prefix".into());
        }
        if !origin.commit.is_empty() {
            hex(&origin.commit, 40)?;
        }
        let length = u64::from_le_bytes(bytes[248..256].try_into().unwrap());
        if length == 0 {
            return if bytes[256..].iter().all(|byte| *byte == 0) {
                Ok((origin, None))
            } else {
                Err("unbound identity region contains unexpected data".into())
            };
        }
        let length = usize::try_from(length).map_err(|_| "identity length exceeds capacity")?;
        if length > SIZE - PAYLOAD || bytes[PAYLOAD + length..].iter().any(|byte| *byte != 0) {
            return Err("identity payload or padding exceeds its declared bounds".into());
        }
        if self.checksum().as_slice() != &bytes[256..288] {
            return Err("identity checksum differs from its content".into());
        }
        let binding: Binding = serde_json::from_slice(&bytes[PAYLOAD..PAYLOAD + length])
            .map_err(|error| format!("cannot decode identity: {error}"))?;
        binding.verify(&origin)?;
        Ok((origin, Some(binding)))
    }

    pub fn encode(&self, binding: &Binding) -> Result<Vec<u8>, String> {
        let bytes = self.0;
        let (origin, held) = self.decode()?;
        binding.verify(&origin)?;
        if let Some(held) = held {
            return if held == *binding {
                Ok(bytes.to_vec())
            } else {
                Err("identity is already bound to another publication".into())
            };
        }
        let payload = serde_json::to_vec(binding).map_err(|error| error.to_string())?;
        if payload.len() > SIZE - PAYLOAD {
            return Err("identity payload exceeds reserved capacity".into());
        }
        let mut result = bytes.to_vec();
        result[248..256].copy_from_slice(&(payload.len() as u64).to_le_bytes());
        result[PAYLOAD..PAYLOAD + payload.len()].copy_from_slice(&payload);
        let digest = Codec(&result).checksum();
        result[256..288].copy_from_slice(&digest);
        Codec(&result).decode()?;
        Ok(result)
    }

    fn checksum(&self) -> Vec<u8> {
        let bytes = self.0;
        let mut sponge = Sha256::new();
        sponge.update(&bytes[..256]);
        sponge.update(&bytes[288..]);
        sponge.finalize().to_vec()
    }
}
fn text(bytes: &[u8]) -> Result<String, String> {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    if bytes[end..].iter().any(|byte| *byte != 0) {
        return Err("identity field contains noncanonical padding".into());
    }
    std::str::from_utf8(&bytes[..end])
        .map(str::to_owned)
        .map_err(|error| error.to_string())
}

fn hex(value: &str, length: usize) -> Result<(), String> {
    if value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        Ok(())
    } else {
        Err(format!(
            "identity requires a lowercase {length}-digit digest"
        ))
    }
}
