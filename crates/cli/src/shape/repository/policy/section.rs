use std::path::Path;

pub(crate) enum Origin {
    File,
    Section,
}

pub(crate) enum State {
    Absent,
    Unread(String),
    Held(Origin, toml::Table),
}

pub(crate) fn present(path: &Path) -> Result<bool, String> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.to_string()),
    }
}

pub(crate) fn read(root: &Path, file: &str, name: &str) -> State {
    let path = root.join(file);
    let exists = match present(&path) {
        Ok(exists) => exists,
        Err(error) => return State::Unread(format!("cannot read {file}: {}", line(error))),
    };
    let held = extract(&root.join("plumb.toml"), name);
    match (exists, held) {
        (true, Ok(Some(_))) => State::Unread(format!(
            "both {file} and [{name}] in plumb.toml exist; keep only one"
        )),
        (true, Err(error)) | (false, Err(error)) => State::Unread(format!(
            "cannot read [{name}] in plumb.toml: {}",
            line(error)
        )),
        (true, Ok(None)) => legacy(&path, file),
        (false, Ok(Some(table))) => State::Held(Origin::Section, table),
        (false, Ok(None)) => State::Absent,
    }
}

fn extract(path: &Path, name: &str) -> Result<Option<toml::Table>, String> {
    if !present(path)? {
        return Ok(None);
    }
    let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    let mut doc = text
        .parse::<toml::Table>()
        .map_err(|error| error.to_string())?;
    match doc.remove(name) {
        None => Ok(None),
        Some(toml::Value::Table(table)) => Ok(Some(table)),
        Some(_) => Err(format!("key `{name}` exists but is not a table")),
    }
}

fn legacy(path: &Path, file: &str) -> State {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => return State::Unread(format!("cannot read {file}: {}", line(error))),
    };
    match text.parse::<toml::Table>() {
        Ok(table) => State::Held(Origin::File, table),
        Err(error) => State::Unread(format!("cannot read {file}: {}", line(error))),
    }
}

fn line(error: impl std::fmt::Display) -> String {
    let text = error.to_string();
    text.lines().next().unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent() {
        let root = tempfile::tempdir().unwrap();
        assert!(matches!(
            read(root.path(), "ectropy.toml", "ectropy"),
            State::Absent
        ));
        std::fs::write(root.path().join("plumb.toml"), "[identity]\nns = 'probe'\n").unwrap();
        assert!(matches!(
            read(root.path(), "ectropy.toml", "ectropy"),
            State::Absent
        ));
        assert!(matches!(
            read(root.path(), "runseal.toml", "runseal"),
            State::Absent
        ));
    }

    #[test]
    fn file() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("ectropy.toml"), "[limit]\nblock = 4\n").unwrap();
        match read(root.path(), "ectropy.toml", "ectropy") {
            State::Held(Origin::File, table) => {
                assert_eq!(table["limit"]["block"].as_integer(), Some(4));
            }
            _ => panic!("expected file origin"),
        }
    }

    #[test]
    fn section() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("plumb.toml"),
            "[ectropy.limit]\nblock = 4\n[runseal]\n",
        )
        .unwrap();
        match read(root.path(), "ectropy.toml", "ectropy") {
            State::Held(Origin::Section, table) => {
                assert_eq!(table["limit"]["block"].as_integer(), Some(4));
            }
            _ => panic!("expected section origin"),
        }
        assert!(matches!(
            read(root.path(), "runseal.toml", "runseal"),
            State::Held(Origin::Section, _)
        ));
    }

    #[test]
    fn conflict() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("ectropy.toml"), "[limit]\nblock = 4\n").unwrap();
        std::fs::write(
            root.path().join("plumb.toml"),
            "[ectropy.limit]\nblock = 4\n",
        )
        .unwrap();
        match read(root.path(), "ectropy.toml", "ectropy") {
            State::Unread(why) => assert!(why.contains("both ectropy.toml and [ectropy]"), "{why}"),
            _ => panic!("expected conflict refusal"),
        }
    }

    #[test]
    fn malformed() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("plumb.toml"), "ectropy = 7\n").unwrap();
        match read(root.path(), "ectropy.toml", "ectropy") {
            State::Unread(why) => {
                assert!(why.contains("cannot read [ectropy] in plumb.toml"), "{why}")
            }
            _ => panic!("expected non-table refusal"),
        }
        std::fs::remove_file(root.path().join("plumb.toml")).unwrap();
        std::fs::create_dir(root.path().join("plumb.toml")).unwrap();
        match read(root.path(), "ectropy.toml", "ectropy") {
            State::Unread(why) => {
                assert!(why.contains("cannot read [ectropy] in plumb.toml"), "{why}")
            }
            _ => panic!("expected unreadable plumb.toml refusal"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn dangling() {
        let root = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(
            root.path().join("missing.toml"),
            root.path().join("plumb.toml"),
        )
        .unwrap();
        match read(root.path(), "ectropy.toml", "ectropy") {
            State::Unread(why) => {
                assert!(why.contains("cannot read [ectropy] in plumb.toml"), "{why}")
            }
            _ => panic!("expected dangling symlink refusal"),
        }
    }
}
