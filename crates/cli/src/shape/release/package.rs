use super::Spec;
use serde::Deserialize;
use std::path::{Component, Path, PathBuf};

const UNITS: &str = "root/lib/systemd/system";

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Apk {
    pub binary: String,
    pub root: PathBuf,
    pub profile: Profile,
}

#[derive(Clone, Debug, Deserialize)]
pub enum Profile {
    #[serde(rename = "openwrt-aarch64-cortex-a53")]
    Openwrt,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Metadata {
    description: String,
    license: String,
    #[serde(default)]
    depends: Vec<String>,
}

impl Apk {
    pub(super) fn validate(&self, spec: &Spec) -> Result<(), String> {
        let triple = match self.profile {
            Profile::Openwrt => "aarch64-unknown-linux-musl",
        };
        if !spec
            .executable(&self.binary)
            .is_some_and(|held| held.carries(triple))
        {
            return Err(format!(
                "OpenWrt APK binary {} must declare target {triple}",
                self.binary
            ));
        }
        let mut root = spec.root.clone();
        if self.root.to_str().is_none_or(|value| {
            value.split('/').any(|part| {
                part.is_empty() || matches!(part, "." | "..") || part.contains(['\\', ':'])
            })
        }) {
            return Err("OpenWrt APK root must name a repository-relative directory".into());
        }
        for part in self.root.components() {
            let Component::Normal(name) = part else {
                return Err(
                    "OpenWrt APK root must name a normalized repository-relative directory".into(),
                );
            };
            root.push(name);
            if root.is_symlink() {
                return Err("OpenWrt APK root must not traverse a symlink".into());
            }
        }
        self.metadata(&root)?;
        self.payload(&root)
    }

    fn metadata(&self, root: &Path) -> Result<(), String> {
        let path = root.join("package.toml");
        if path.is_symlink() || !path.is_file() {
            return Err("OpenWrt APK requires a regular package.toml metadata file".into());
        }
        let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
        let held: Metadata =
            toml::from_str(&text).map_err(|error| format!("OpenWrt APK metadata: {error}"))?;
        for value in [&held.description, &held.license] {
            if value.trim().is_empty() || value.chars().any(char::is_control) {
                return Err(
                    "OpenWrt APK description and license must be nonempty single-line values"
                        .into(),
                );
            }
        }
        let mut seen = std::collections::BTreeSet::new();
        for name in held.depends {
            super::token("OpenWrt APK dependency", &name, false)?;
            if !seen.insert(name) {
                return Err("OpenWrt APK dependencies must be distinct".into());
            }
        }
        Ok(())
    }

    fn payload(&self, root: &Path) -> Result<(), String> {
        let mut pending = vec![root.to_path_buf()];
        while let Some(directory) = pending.pop() {
            for entry in std::fs::read_dir(directory).map_err(|error| error.to_string())? {
                let entry = entry.map_err(|error| error.to_string())?;
                let path = entry.path();
                let relative = path.strip_prefix(root).map_err(|error| error.to_string())?;
                let kind = entry.file_type().map_err(|error| error.to_string())?;
                if relative == Path::new("package.toml") && kind.is_file() {
                    continue;
                }
                if !relative.starts_with("root") || (!kind.is_dir() && !kind.is_file()) {
                    return Err("OpenWrt APK root holds package.toml and regular root/ payload only; scripts and links refuse".into());
                }
                if relative == Path::new("root") && !kind.is_dir() {
                    return Err("OpenWrt APK root/ must be a directory".into());
                }
                if relative.starts_with("root/etc/rc.d")
                    || relative == Path::new(&format!("root/usr/bin/{}", self.binary))
                {
                    return Err("OpenWrt APK payload cannot enable services or replace its bound executable".into());
                }
                if kind.is_dir() {
                    pending.push(path);
                }
            }
        }
        Ok(())
    }
}

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
