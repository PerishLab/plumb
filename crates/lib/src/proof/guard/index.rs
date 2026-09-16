use super::store;
use std::path::Path;

pub fn tree(root: &Path) -> Result<String, String> {
    let path = store::git(
        root,
        &["rev-parse", "--path-format=absolute", "--git-path", "index"],
        "locate staged index",
    )?;
    let source = Path::new(&path);
    let bytes = match std::fs::read(source) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return store::git(root, &["mktree"], "read empty staged tree");
        }
        Err(error) => return Err(format!("cannot read staged index: {error}")),
    };
    let parent = source.parent().ok_or("staged index has no parent")?;
    let index = tempfile::Builder::new()
        .prefix("plumb-index-")
        .tempfile_in(parent)
        .map_err(|error| format!("cannot reserve staged index snapshot: {error}"))?;
    std::fs::write(index.path(), bytes)
        .map_err(|error| format!("cannot copy staged index: {error}"))?;
    let output = crate::config::detached("git")
        .arg("-C")
        .arg(root)
        .arg("write-tree")
        .env("GIT_INDEX_FILE", index.path())
        .output()
        .map_err(|error| format!("cannot read staged tree snapshot: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cannot read staged tree snapshot: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().into())
}
