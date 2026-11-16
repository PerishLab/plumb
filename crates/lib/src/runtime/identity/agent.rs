#[doc = "An explicit executable/version HTTP User-Agent. Constructing it sends no request or header."]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Agent(String);

impl Agent {
    #[doc = "Requires nonempty HTTP tokens for the executable name and version; neither value is sanitized or inferred from the host."]
    pub fn new(product: &str, version: &str) -> Result<Self, String> {
        if !Self::token(product) || !Self::token(version) {
            return Err("User-Agent product and version must be nonempty HTTP tokens".into());
        }
        Ok(Self(format!("{product}/{version}")))
    }

    #[doc = "The same executable/version value emitted by Display."]
    pub fn value(&self) -> &str {
        &self.0
    }

    fn token(value: &str) -> bool {
        !value.is_empty()
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte))
    }
}

impl std::fmt::Display for Agent {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        output.write_str(self.value())
    }
}
