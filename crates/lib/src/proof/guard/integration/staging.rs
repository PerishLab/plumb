use std::path::{Path, PathBuf};

pub const PREFIX: &str = "guard-";

pub fn root() -> Option<PathBuf> {
    crate::config::value("PLUMB_HOME")
        .map(PathBuf::from)
        .or_else(|| crate::config::data("plumb"))
        .map(|home| home.join("tmp"))
}

pub fn contains(root: &Path, path: &Path) -> bool {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let path = path.canonicalize().unwrap_or_else(|_| {
        path.parent()
            .and_then(|parent| parent.canonicalize().ok())
            .zip(path.file_name())
            .map(|(parent, name)| parent.join(name))
            .unwrap_or_else(|| path.to_path_buf())
    });
    path.parent() == Some(root.as_path())
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(PREFIX))
}

#[cfg(test)]
mod tests {
    use super::contains;

    #[test]
    fn staged() {
        let root = tempfile::tempdir().expect("root");
        let staged = root.path().join("guard-a1b2");
        std::fs::create_dir(&staged).expect("staged");
        assert!(contains(root.path(), &staged));
        assert!(contains(root.path(), &root.path().join("guard-gone")));
    }

    #[test]
    fn foreign() {
        let root = tempfile::tempdir().expect("root");
        let other = tempfile::tempdir().expect("other");
        assert!(!contains(root.path(), &other.path().join("guard-a1b2")));
        assert!(!contains(root.path(), &root.path().join("linked")));
        assert!(!contains(
            root.path(),
            &root.path().join("nested/guard-a1b2")
        ));
        assert!(!contains(root.path(), root.path()));
    }
}
