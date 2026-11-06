use sha2::{Digest, Sha256};

pub(super) struct Wire;

impl Wire {
    pub fn digest(bytes: &[u8]) -> String {
        format!("{:x}", Sha256::digest(bytes))
    }

    pub fn hex(value: &str, size: usize) -> Result<(), String> {
        if value.len() != size
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(format!(
                "expected a lowercase hexadecimal identity of {size} digits"
            ));
        }
        Ok(())
    }

    pub fn name(value: &str) -> Result<(), String> {
        if value.is_empty()
            || value.len() > 128
            || !value.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
            })
        {
            return Err("expected a nonempty bounded identifier".into());
        }
        Ok(())
    }

    pub fn path(value: &str) -> Result<(), String> {
        if value.is_empty() || value.len() > 255 || value.split('/').count() > 16 {
            return Err("material name exceeds its path bounds".into());
        }
        for part in value.split('/') {
            if part.is_empty()
                || matches!(part, "." | "..")
                || !part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
            {
                return Err("material names require canonical relative paths".into());
            }
        }
        Ok(())
    }

    pub fn reference(value: &str) -> Result<(), String> {
        if value.len() > 2048
            || value
                .chars()
                .any(|character| character.is_whitespace() || matches!(character, '?' | '@' | '\\'))
            || !value.contains("#issuecomment-")
        {
            return Err("review references require an explicit PerishLab Issue comment URL".into());
        }
        let (location, comment) = value
            .split_once("#issuecomment-")
            .ok_or("review reference has no comment identity")?;
        let path = location
            .strip_prefix("https://github.com/PerishLab/")
            .ok_or("review reference belongs to another authority")?;
        let parts: Vec<_> = path.split('/').collect();
        if parts.len() != 3 {
            return Err("review reference requires repository, issues and number".into());
        }
        Self::path(parts[0])?;
        if parts[1] != "issues" {
            return Err("review reference must identify an Issue".into());
        }
        if !Self::number(parts[2]) || !Self::number(comment) {
            return Err("review references require exact Issue and comment identities".into());
        }
        Ok(())
    }

    fn number(value: &str) -> bool {
        value
            .parse::<u64>()
            .is_ok_and(|number| number > 0 && number.to_string() == value)
    }

    pub fn turn(value: &str) -> bool {
        !value.is_empty()
            && !value.starts_with('0')
            && value.bytes().all(|byte| byte.is_ascii_digit())
    }
}
