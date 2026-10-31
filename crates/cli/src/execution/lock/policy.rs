use std::path::Path;

pub(super) const PREFIX: &str = "plumb.lock/";
pub(super) const METHODS: [&str; 5] = [
    "std::fs::File::lock",
    "std::fs::File::lock_shared",
    "std::fs::File::try_lock",
    "std::fs::File::try_lock_shared",
    "std::fs::File::unlock",
];

pub(super) fn configuration(root: &Path) -> Result<String, String> {
    let files = [root.join("clippy.toml"), root.join(".clippy.toml")];
    let found = files
        .iter()
        .filter(|path| path.exists())
        .collect::<Vec<_>>();
    if found.len() > 1 {
        return Err("lock resolve refuses conflicting clippy.toml and .clippy.toml".into());
    }
    let text = found
        .first()
        .map(std::fs::read_to_string)
        .transpose()
        .map_err(|error| format!("cannot read Clippy configuration: {error}"))?
        .unwrap_or_default();
    let mut config = text
        .parse::<toml::Table>()
        .map_err(|error| format!("cannot parse Clippy configuration: {error}"))?;
    let entries = config
        .entry("disallowed-methods")
        .or_insert_with(|| toml::Value::Array(Vec::new()))
        .as_array_mut()
        .ok_or("Clippy disallowed-methods must be an array")?;
    entries.retain(|entry| {
        let path = entry
            .as_str()
            .or_else(|| entry.get("path").and_then(toml::Value::as_str));
        !path.is_some_and(|path| METHODS.contains(&path))
    });
    for path in METHODS {
        let entry = toml::Table::from_iter([
            ("path".into(), toml::Value::String(path.into())),
            (
                "reason".into(),
                toml::Value::String(format!("{PREFIX}{path}")),
            ),
        ]);
        entries.push(toml::Value::Table(entry));
    }
    toml::to_string(&config).map_err(|error| error.to_string())
}
