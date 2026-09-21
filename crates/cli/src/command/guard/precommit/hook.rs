use std::path::{Path, PathBuf};

use super::tree;

const HOOKS: [(&str, &str); 2] = [
    (
        "pre-commit",
        plumb::seat::resource!("assets/git/hooks/pre-commit"),
    ),
    (
        "commit-msg",
        plumb::seat::resource!("assets/git/hooks/commit-msg"),
    ),
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
        if !self.governed()? {
            return Ok(None);
        }
        self.write(&HOOKS).map(Some)
    }

    fn governed(&self) -> Result<bool, String> {
        Ok(self.0.join("plumb.toml").is_file())
    }

    fn write(&self, bodies: &[(&str, &str)]) -> Result<String, String> {
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

fn inspect(path: &Path) -> Option<Finding> {
    match std::fs::metadata(path) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Some(Finding::Wrong(format!(
                "{} is absent; run plumb configuration install",
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
                "{} is not executable; run plumb configuration install",
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
