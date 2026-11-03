use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

pub(crate) fn inspect(root: &Path, command: &Command) -> Result<Option<String>, String> {
    if !cfg!(target_os = "linux") {
        return Ok(Some(
            "Cargo launch-context capture is qualified on Linux only".into(),
        ));
    }
    if command
        .get_envs()
        .any(|(key, value)| value.is_some() && environment(key))
    {
        return Ok(Some(
            "Cargo execution already selects a runner or target".into(),
        ));
    }
    if std::env::vars_os().any(|(key, _)| {
        environment(&key)
            && !command
                .get_envs()
                .any(|(changed, value)| changed == key && value.is_none())
    }) {
        return Ok(Some(
            "ambient Cargo execution selects a runner or target".into(),
        ));
    }
    if foreign(root, command)? {
        return Ok(Some(
            "Cargo has configuration outside the source root".into(),
        ));
    }
    for name in ["config", "config.toml"] {
        let path = root.join(".cargo").join(name);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(format!("cannot read Cargo configuration: {error}")),
        };
        let value: toml::Value = toml::from_str(&text)
            .map_err(|error| format!("invalid Cargo configuration: {error}"))?;
        if configured(&value) {
            return Ok(Some(
                "source Cargo configuration selects a runner, target, or included configuration"
                    .into(),
            ));
        }
    }
    Ok(None)
}

fn environment(key: &std::ffi::OsStr) -> bool {
    let Some(key) = key.to_str() else { return true };
    if matches!(key, "CARGO_BUILD_TARGET" | "CARGO_TARGET_APPLIES_TO_HOST")
        || key.starts_with("CARGO_UNSTABLE_")
    {
        return true;
    }
    key.strip_prefix("CARGO_")
        .is_some_and(|key| key.ends_with("_RUNNER"))
}

pub(super) fn foreign(root: &Path, command: &Command) -> Result<bool, String> {
    let physical = super::context::directory(root)?;
    let mut directories = physical
        .ancestors()
        .skip(1)
        .map(|parent| parent.join(".cargo"))
        .collect::<Vec<_>>();
    let home = input(command, "CARGO_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            input(command, "HOME").map(|home| std::path::PathBuf::from(home).join(".cargo"))
        });
    if let Some(home) = home {
        directories.push(root.join(home));
    }
    for directory in directories {
        for name in ["config", "config.toml"] {
            match std::fs::symlink_metadata(directory.join(name)) {
                Ok(_) => return Ok(true),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
                Err(error) => return Err(format!("cannot inspect Cargo configuration: {error}")),
            }
        }
    }
    Ok(false)
}

fn input(command: &Command, name: &str) -> Option<OsString> {
    command
        .get_envs()
        .find(|(key, _)| *key == name)
        .map(|(_, value)| value.map(std::ffi::OsStr::to_os_string))
        .unwrap_or_else(|| std::env::var_os(name))
}

pub(super) fn configured(value: &toml::Value) -> bool {
    let Some(table) = value.as_table() else {
        return false;
    };
    table.iter().any(|(key, value)| {
        matches!(
            key.as_str(),
            "runner" | "target" | "include" | "unstable" | "host" | "target-applies-to-host"
        ) || environment(std::ffi::OsStr::new(key))
            || configured(value)
    })
}
