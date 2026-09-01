use std::path::{Component, Path};

pub(super) struct Value<'a>(pub &'a str);

impl Value<'_> {
    pub fn source(&self) -> Result<(), String> {
        if !self.0.starts_with("https://") || self.0.len() <= "https://".len() {
            return Err(format!("depot source must be an https URL: {}", self.0));
        }
        Ok(())
    }

    pub fn digest(&self, name: &str) -> Result<(), String> {
        if self.0.len() != 64
            || !self
                .0
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(format!(
                "{name} is not a lowercase sha256 digest: {}",
                self.0
            ));
        }
        Ok(())
    }

    pub fn anchored(&self, name: &str) -> Result<(), String> {
        if self.0.is_empty() || self.0.contains('\\') {
            return Err(format!("{name} is not anchored: {}", self.0));
        }
        if Path::new(self.0)
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(format!("{name} is not anchored: {}", self.0));
        }
        Ok(())
    }

    pub fn component(&self, name: &str) -> Result<(), String> {
        self.anchored(name)?;
        if Path::new(self.0).components().count() != 1 {
            return Err(format!("{name} is not one path component: {}", self.0));
        }
        Ok(())
    }
}

pub(super) fn version(raw: &str) -> Result<(), String> {
    Value(raw).component("release version")?;
    semver::Version::parse(raw.trim_start_matches('v'))
        .map(|_| ())
        .map_err(|error| format!("cannot parse release version {raw}: {error}"))
}

pub(super) fn timestamp(raw: &str) -> Result<(), String> {
    let bytes = raw.as_bytes();
    let punctuation = [
        (4, b'-'),
        (7, b'-'),
        (10, b'T'),
        (13, b':'),
        (16, b':'),
        (19, b'Z'),
    ];
    if bytes.len() != 20
        || punctuation.iter().any(|(at, byte)| bytes[*at] != *byte)
        || bytes.iter().enumerate().any(|(at, byte)| {
            !punctuation.iter().any(|(held, _)| *held == at) && !byte.is_ascii_digit()
        })
    {
        return Err(format!("invalid depot creation time: {raw}"));
    }
    Ok(())
}
