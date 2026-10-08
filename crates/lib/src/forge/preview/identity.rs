pub(super) struct Identity<'a>(pub &'a str);

impl Identity<'_> {
    pub fn hex(&self, size: usize) -> Result<(), String> {
        let value = self.0;
        if value.len() != size
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        {
            return Err(format!(
                "Preview identity needs {size} lowercase hex digits"
            ));
        }
        Ok(())
    }

    pub fn slug(&self) -> Result<(), String> {
        let value = self.0;
        if value.len() > 48 || !value.starts_with(|char: char| char.is_ascii_lowercase()) {
            return Err("Preview name needs a lowercase slug of at most 48 bytes".into());
        }
        if value.split('-').any(|part| {
            part.is_empty()
                || !part
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        }) {
            return Err("Preview name has malformed slug segments".into());
        }
        Ok(())
    }

    pub fn repository(&self) -> Result<(), String> {
        let value = self.0;
        let parts: Vec<_> = value.split('/').collect();
        if parts.len() != 2 || parts.iter().any(|part| part.is_empty()) {
            return Err("Preview repository needs one owner/name coordinate".into());
        }
        if parts.iter().any(|part| {
            !part
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        }) {
            return Err("Preview repository has malformed coordinate segments".into());
        }
        Ok(())
    }

    pub fn text(&self, limit: usize) -> Result<(), String> {
        let value = self.0;
        if value.trim().is_empty()
            || value.chars().count() > limit
            || value.chars().any(|char| char < '\u{20}')
        {
            return Err("Preview text must be nonblank, bounded and free of control bytes".into());
        }
        Ok(())
    }

    pub fn origin(&self) -> Result<(), String> {
        let value = self.0;
        self.text(512)?;
        let host = value
            .strip_prefix("https://")
            .ok_or("Preview origin must use HTTPS")?;
        let host = host.strip_suffix('/').unwrap_or(host);
        let parts: Vec<_> = host.split('.').collect();
        if parts.len() != 4 || parts[2..] != ["workers", "dev"] {
            return Err("Preview origin must be a native workers.dev origin".into());
        }
        if parts[..2].iter().any(|part| {
            part.is_empty()
                || !part
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        }) {
            return Err("Preview origin has malformed native labels".into());
        }
        Ok(())
    }
}
