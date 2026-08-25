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
        if self.0.len() != 64 || !self.0.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(format!("{name} is not a sha256 digest: {}", self.0));
        }
        Ok(())
    }

    pub fn commit(&self, name: &str) -> Result<(), String> {
        if !(7..=64).contains(&self.0.len()) {
            return Err(format!("{name} is not a git object id: {}", self.0));
        }
        if !self.0.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(format!("{name} is not a git object id: {}", self.0));
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
