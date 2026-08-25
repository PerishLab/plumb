use super::Spec;

impl Spec {
    pub fn binary(&self) -> bool {
        !self.binaries.is_empty() || !self.target.is_empty()
    }

    pub fn attached(&self) -> bool {
        [
            self.cargo.is_some(),
            self.oci.is_some(),
            self.chart.is_some(),
            self.npm.is_some(),
            self.cfworker.is_some(),
        ]
        .into_iter()
        .any(|held| held)
    }

    pub fn surface(&self) -> Vec<&'static str> {
        [
            (!self.binaries.is_empty(), "binary"),
            (self.cargo.is_some(), "cargo"),
            (self.chart.is_some(), "chart"),
            (self.npm.is_some(), "npm"),
            (self.cfworker.is_some(), "cfworker"),
            (self.oci.is_some(), "oci"),
        ]
        .into_iter()
        .filter(|(present, _)| *present)
        .map(|(_, name)| name)
        .collect()
    }

    pub(super) fn standalone(&self) -> Result<(), String> {
        if self.skill || self.deb.is_some() {
            return Err("a skill or Debian attachment requires a binary release".into());
        }
        if self.attached() {
            return Ok(());
        }
        Err("a release declares a binary shape or at least one attachment".into())
    }
}
