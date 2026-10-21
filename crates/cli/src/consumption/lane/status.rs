use super::Status;
use plumb::lane::evidence::{Observation, Reason};
use plumb::lane::{Name, Source};
use std::path::{Path, PathBuf};

pub(super) fn read(options: Status) -> Result<serde_json::Value, String> {
    let root = PathBuf::from(options.target.root)
        .canonicalize()
        .map_err(|_| "cannot resolve lane repository root")?;
    let top = PathBuf::from(git(&root, &["rev-parse", "--show-toplevel"])?)
        .canonicalize()
        .map_err(|_| "cannot resolve lane Git root")?;
    if top != root {
        return Err("lane status requires an exact repository root".into());
    }
    let before = source(&root)?;
    let snapshot = snapshot(&root, &before.0)?;
    let declaration = crate::shape::product::preview::Seat::new(&root).select(
        &snapshot,
        &options.selection.app,
        &Name::read(&options.selection.lane)?,
    )?;
    if let Some(action) = options.require {
        declaration.capabilities().require(action)?;
    }
    if source(&root)? != before {
        return Err("lane source moved during observation".into());
    }
    let source = Source::new(declaration.target().repository().into(), before.0, before.1)?;
    Ok(serde_json::json!({
        "schema": "plumb.lane.status/v1", "declaration": declaration, "source": source,
        "current": Observation::Unknown { reason: Reason::Unverified }, "artifact": null
    }))
}

fn source(root: &Path) -> Result<(String, String), String> {
    if !git(root, &["status", "--porcelain", "--untracked-files=all"])?.is_empty() {
        return Err(
            "lane status requires clean exact source; no build or repair is performed".into(),
        );
    }
    Ok((
        git(root, &["rev-parse", "--verify", "HEAD"])?,
        git(root, &["rev-parse", "--verify", "HEAD^{tree}"])?,
    ))
}

fn snapshot(root: &Path, commit: &str) -> Result<plumb::snapshot::Snapshot, String> {
    let snapshot = plumb::snapshot::Snapshot::staged(root).map_err(|error| error.to_string())?;
    let listed = git(root, &["ls-tree", "--full-tree", "-r", "-z", commit])?;
    let expected = listed
        .split('\0')
        .filter(|line| !line.is_empty())
        .map(entry)
        .collect::<Result<Vec<_>, _>>()?;
    let actual: Vec<_> = snapshot
        .entries()
        .iter()
        .map(|entry| format!("{} {}\t{}", entry.mode(), entry.oid(), entry.path()))
        .collect();
    if actual != expected {
        return Err("lane source snapshot differs from the exact committed tree".into());
    }
    Ok(snapshot)
}

fn entry(line: &str) -> Result<String, String> {
    let (metadata, path) = line.split_once('\t').ok_or("invalid committed lane tree")?;
    let fields: Vec<_> = metadata.split(' ').collect();
    if fields.len() != 3 {
        return Err("invalid committed lane tree".into());
    }
    if !matches!(fields[1], "blob" | "commit") {
        return Err("invalid committed lane tree entry".into());
    }
    Ok(format!("{} {}\t{}", fields[0], fields[2], path))
}

fn git(root: &Path, arguments: &[&str]) -> Result<String, String> {
    let output = plumb::config::detached("git")
        .arg("--no-optional-locks")
        .arg("-C")
        .arg(root)
        .args(arguments)
        .output()
        .map_err(|_| "cannot observe lane Git source")?;
    if !output.status.success() {
        return Err("cannot observe lane Git source".into());
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_string())
        .map_err(|_| "invalid lane Git source observation".into())
}

#[cfg(test)]
mod tests {
    #[test]
    fn committed() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("plumb.toml");
        std::fs::write(&file, "original").unwrap();
        super::git(root.path(), &["init", "--quiet"]).unwrap();
        super::git(root.path(), &["add", "."]).unwrap();
        super::git(
            root.path(),
            &[
                "-c",
                "user.name=fixture",
                "-c",
                "user.email=fixture@example.test",
                "-c",
                "core.hooksPath=",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "--quiet",
                "-m",
                "fixture",
            ],
        )
        .unwrap();
        let commit = super::git(root.path(), &["rev-parse", "HEAD"]).unwrap();
        std::fs::write(&file, "transient").unwrap();
        let snapshot = super::snapshot(root.path(), &commit).unwrap();
        assert_eq!(snapshot.entries()[0].bytes(), b"original");
        super::git(root.path(), &["add", "."]).unwrap();
        assert!(super::snapshot(root.path(), &commit).is_err());
        assert_eq!(std::fs::read_to_string(file).unwrap(), "transient");
    }
}
