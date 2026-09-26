use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Depot {
    pub source: String,
}

impl Depot {
    pub(super) fn validate(&self) -> Result<(), String> {
        plumb::depot::v3::check(&self.source)
    }
}

impl super::Spec {
    pub fn source(&self) -> String {
        self.depot.as_ref().map_or_else(
            || format!("https://depot.{}.perish.uk", self.product),
            |held| held.source.clone(),
        )
    }
}
