#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Agent(String);

impl Agent {
    pub fn new(product: &str, version: &str) -> Result<Self, String> {
        if !Self::token(product) || !Self::token(version) {
            return Err("User-Agent product and version must be nonempty HTTP tokens".into());
        }
        Ok(Self(format!("{product}/{version}")))
    }

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
