use super::{Reference, Verified};

pub const LIMIT: usize = 65536;

#[derive(Clone, Debug)]
pub struct Reader {
    origin: Origin,
    reference: Reference,
}

impl Reader {
    pub fn new(origin: &str, reference: Reference) -> Result<Self, String> {
        Ok(Self {
            origin: Origin::read(origin)?,
            reference,
        })
    }

    pub fn url(&self) -> String {
        format!(
            "{}/lane/v1/evidence/{}.json",
            self.origin.0,
            self.reference.digest().value()
        )
    }

    pub fn verify(&self, bytes: &[u8]) -> Result<Verified, String> {
        if bytes.len() > LIMIT {
            return Err("lane public evidence exceeds its document budget".into());
        }
        self.reference
            .verify(&self.reference, bytes)
            .map_err(|_| "lane public evidence is unverified".into())
    }

    #[cfg(feature = "depot")]
    pub fn read(&self) -> Result<Verified, String> {
        use std::io::Read;
        use std::time::Duration;
        use ureq::ResponseExt;

        let config = ureq::Agent::config_builder()
            .https_only(true)
            .proxy(None)
            .max_redirects(0)
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(30)))
            .build();
        let agent = ureq::Agent::new_with_config(config);
        let url = self.url();
        let mut response = agent
            .get(&url)
            .header("Accept", "application/json")
            .header("Accept-Encoding", "identity")
            .call()
            .map_err(|_| "lane public evidence is unavailable")?;
        if response.status().as_u16() != 200
            || response.get_uri().to_string() != url
            || response.headers().contains_key("Content-Range")
        {
            return Err("lane public evidence response is unverified".into());
        }
        let mut bytes = Vec::new();
        response
            .body_mut()
            .as_reader()
            .take((LIMIT + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| "lane public evidence is unavailable")?;
        self.verify(&bytes)
    }
}

#[derive(Clone, Debug)]
struct Origin(String);

impl Origin {
    fn read(value: &str) -> Result<Self, String> {
        let host = value
            .strip_prefix("https://")
            .ok_or("lane public evidence requires a bare HTTPS origin")?;
        let domain = Domain::read(host)?;
        Ok(Self(format!("https://{}", domain.name())))
    }
}

struct Domain(Vec<Label>);

impl Domain {
    fn read(value: &str) -> Result<Self, String> {
        let labels = value
            .split('.')
            .map(Label::read)
            .collect::<Result<Vec<_>, _>>()?;
        if value.len() > 253 || labels.len() < 2 {
            return Err("lane public evidence requires a bounded DNS origin".into());
        }
        if !labels.last().is_some_and(Label::public) {
            return Err("lane public evidence requires a public DNS suffix".into());
        }
        Ok(Self(labels))
    }

    fn name(&self) -> String {
        self.0
            .iter()
            .map(|label| label.0.as_str())
            .collect::<Vec<_>>()
            .join(".")
    }
}

struct Label(String);

impl Label {
    fn read(value: &str) -> Result<Self, String> {
        if value.is_empty() || value.len() > 63 {
            return Err("lane public evidence requires a bounded DNS label".into());
        }
        if !value.starts_with(|value: char| value.is_ascii_alphanumeric())
            || !value.ends_with(|value: char| value.is_ascii_alphanumeric())
        {
            return Err("lane public evidence requires alphanumeric DNS label edges".into());
        }
        if !value
            .bytes()
            .all(|value| value.is_ascii_lowercase() || value.is_ascii_digit() || value == b'-')
        {
            return Err("lane public evidence requires normalized DNS labels".into());
        }
        Ok(Self(value.into()))
    }

    fn public(&self) -> bool {
        self.0.len() >= 2
            && self.0.bytes().all(|value| value.is_ascii_lowercase())
            && !matches!(self.0.as_str(), "local" | "localhost")
    }
}
