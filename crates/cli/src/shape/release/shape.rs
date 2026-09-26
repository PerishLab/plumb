use super::Spec;

impl Spec {
    pub fn ship(&self) -> Result<(), String> {
        for target in &self.target {
            if !matches!(
                target.triple.as_str(),
                "x86_64-unknown-linux-gnu" | "aarch64-apple-darwin" | "x86_64-pc-windows-msvc"
            ) {
                return Err(format!("unsupported Ship target {}", target.triple));
            }
        }
        Ok(())
    }

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
        if self.skill {
            return Err("a skill requires a binary release, whose CLI installs it".into());
        }
        if self.attached() {
            return Ok(());
        }
        Err("a release declares a binary shape or at least one attachment".into())
    }
}
