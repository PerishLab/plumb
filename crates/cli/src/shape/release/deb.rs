use super::Spec;
use serde::Deserialize;
use std::path::{Component, Path, PathBuf};

const UNITS: &str = "root/lib/systemd/system";

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Deb {
    pub binary: String,
    pub root: PathBuf,
}

impl Deb {
    pub(super) fn validate(&self, spec: &Spec) -> Result<(), String> {
        spec.server("Debian placement", &self.binary)?;
        let relative = self
            .root
            .components()
            .all(|held| matches!(held, Component::Normal(_) | Component::CurDir));
        if !relative {
            return Err(format!(
                "Debian placement root must be repository-relative: {}",
                self.root.display()
            ));
        }
        let root = spec.root.join(&self.root);
        self.control(&root)?;
        self.unit(&root)
    }

    fn control(&self, root: &Path) -> Result<(), String> {
        let path = root.join("control");
        let text = std::fs::read_to_string(&path).map_err(|_| {
            format!(
                "Debian placement has no control template: {}",
                self.root.join("control").display()
            )
        })?;
        let count = text.matches("__VERSION__").count();
        if count != 1 {
            return Err(format!(
                "Debian control template carries __VERSION__ {count} times; it carries it once"
            ));
        }
        let package = text
            .lines()
            .find_map(|line| line.strip_prefix("Package:"))
            .map(str::trim);
        if package != Some(self.binary.as_str()) {
            return Err(format!(
                "Debian control Package is {}; it names binary {}",
                package.unwrap_or("absent"),
                self.binary
            ));
        }
        Ok(())
    }

    fn unit(&self, root: &Path) -> Result<(), String> {
        let units: Vec<String> = std::fs::read_dir(root.join(UNITS))
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|held| held == "service"))
            .filter_map(|path| std::fs::read_to_string(path).ok())
            .collect();
        if units.is_empty() {
            return Err(format!(
                "Debian placement has no systemd unit: {}/*.service",
                self.root.join(UNITS).display()
            ));
        }
        let executable = format!("/usr/bin/{}", self.binary);
        let runs = |text: &String| {
            text.lines()
                .filter_map(|line| line.trim().strip_prefix("ExecStart="))
                .any(|command| command.split_whitespace().next() == Some(executable.as_str()))
        };
        if units.iter().any(runs) {
            Ok(())
        } else {
            Err(format!("no Debian systemd unit runs {executable}"))
        }
    }
}
