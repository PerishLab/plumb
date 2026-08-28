use std::path::{Path, PathBuf};

use super::tree;

pub(super) fn install(root: &Path) -> Result<String, String> {
    let located = PathBuf::from(tree::git(
        root,
        &["rev-parse", "--git-path", "hooks"],
        "locate Git hooks",
    )?);
    let hooks = if located.is_absolute() {
        located
    } else {
        root.join(located)
    };
    std::fs::create_dir_all(&hooks)
        .map_err(|error| format!("cannot create {}: {error}", hooks.display()))?;
    hook(
        &hooks.join("pre-commit"),
        "#!/bin/sh\nexec plumb precommit .\n",
    )?;
    hook(
        &hooks.join("commit-msg"),
        "#!/bin/sh\nexec plumb precommit . --attach \"$1\"\n",
    )?;
    Ok(format!(
        "installed Plumb pre-commit and commit-msg hooks at {}",
        hooks.display()
    ))
}

fn hook(path: &Path, body: &str) -> Result<(), String> {
    if path.exists() {
        let held = std::fs::read_to_string(path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        if held != body {
            return Err(format!("{} is owned by another hook", path.display()));
        }
        return Ok(());
    }
    std::fs::write(path, body)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
            .map_err(|error| format!("cannot make {} executable: {error}", path.display()))?;
    }
    Ok(())
}
