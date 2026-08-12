use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Retire {
    pub bucket: String,
    pub zone: String,
}

impl Retire {
    pub fn validate(&self) -> Result<(), String> {
        if !bucketed(&self.bucket) {
            return Err(format!("invalid retirement bucket: {}", self.bucket));
        }
        if !zoned(&self.zone) {
            return Err(format!("invalid retirement zone: {}", self.zone));
        }
        Ok(())
    }
}

fn bucketed(name: &str) -> bool {
    if !(3..=63).contains(&name.len()) {
        return false;
    }
    if name.starts_with('-') || name.ends_with('-') {
        return false;
    }
    name.bytes()
        .all(|held| held.is_ascii_lowercase() || held.is_ascii_digit() || held == b'-')
}

fn zoned(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|held| held.is_ascii_digit() || (b'a'..=b'f').contains(&held))
}
