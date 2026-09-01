use std::path::{Path, PathBuf};

use super::tree;

const HOOKS: [(&str, &str); 2] = [
    ("pre-commit", "assets/git/hooks/pre-commit"),
    ("commit-msg", "assets/git/hooks/commit-msg"),
];

pub(crate) enum Finding {
    Wrong(String),
    Blind(String),
}

pub(crate) fn project(root: &Path) -> Result<Option<String>, String> {
    Hooks(root).project()
}

pub(crate) fn audit(root: &Path) -> Vec<Finding> {
    Hooks(root).audit()
}

struct Hooks<'a>(&'a Path);

impl Hooks<'_> {
    fn project(&self) -> Result<Option<String>, String> {
        if !self.0.join("plumb.toml").is_file() {
            return Ok(None);
        }
        let bodies = HOOKS
            .into_iter()
            .map(|(name, source)| carried(source).map(|body| (name, body)))
            .collect::<Result<Vec<_>, _>>()?;
        self.write(&bodies).map(Some)
    }

    fn write(&self, bodies: &[(&str, String)]) -> Result<String, String> {
        let hooks = self.locate()?;
        std::fs::create_dir_all(&hooks)
            .map_err(|error| format!("cannot create {}: {error}", hooks.display()))?;
        for (name, body) in bodies {
            let target = hooks.join(name);
            std::fs::write(&target, body)
                .map_err(|error| format!("cannot project {}: {error}", target.display()))?;
            executable(&target)?;
        }
        Ok(format!(
            "projected Plumb guard hooks into {}",
            hooks.display()
        ))
    }

    fn audit(&self) -> Vec<Finding> {
        if plumb::config::value("PLUMB_DEPOT_SNAPSHOT").is_some() {
            return HOOKS
                .into_iter()
                .filter_map(|(_, source)| match carried(source) {
                    Ok(body) if !body.is_empty() => None,
                    Ok(_) => Some(Finding::Wrong(format!(
                        "the staged depot snapshot carries empty {source}"
                    ))),
                    Err(error) => Some(Finding::Wrong(error)),
                })
                .collect();
        }
        let hooks = match self.locate() {
            Ok(hooks) => hooks,
            Err(error) => return vec![Finding::Blind(error)],
        };
        HOOKS
            .into_iter()
            .filter_map(|(name, _)| inspect(&hooks.join(name)))
            .collect()
    }

    fn locate(&self) -> Result<PathBuf, String> {
        let located = PathBuf::from(tree::git(
            self.0,
            &["rev-parse", "--git-path", "hooks"],
            "locate Git hooks",
        )?);
        Ok(if located.is_absolute() {
            located
        } else {
            self.0.join(located)
        })
    }
}

fn carried(path: &str) -> Result<String, String> {
    let body = plumb::depot::rules()?.read(path)?;
    if body.is_empty() {
        Err(format!("the active depot carries no {path}"))
    } else {
        Ok(body)
    }
}

fn inspect(path: &Path) -> Option<Finding> {
    match std::fs::metadata(path) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Some(Finding::Wrong(format!(
                "{} is absent; run plumb depot sync",
                path.display()
            )));
        }
        Err(error) => {
            return Some(Finding::Blind(format!(
                "cannot read {}: {error}",
                path.display()
            )));
        }
    }
    #[cfg(unix)]
    if let Ok(metadata) = std::fs::metadata(path) {
        use std::os::unix::fs::PermissionsExt as _;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Some(Finding::Wrong(format!(
                "{} is not executable; run plumb depot sync",
                path.display()
            )));
        }
    }
    None
}

#[cfg(unix)]
fn executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .map_err(|error| format!("cannot make {} executable: {error}", path.display()))?;
    Ok(())
}

#[cfg(not(unix))]
fn executable(_: &Path) -> Result<(), String> {
    Ok(())
}
