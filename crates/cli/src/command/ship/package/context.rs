use crate::shape::release::Spec;
use plumb::snapshot::{Entry, Snapshot};
use std::collections::BTreeSet;
use std::fs::File;
use std::io::{Seek, SeekFrom};
use std::path::{Component, Path};

pub(in crate::command) fn inputs(spec: &Spec) -> Result<Vec<String>, String> {
    if spec.binary() {
        return Ok(vec!["*".into()]);
    }
    let mut paths = BTreeSet::from(["Containerfile".to_string()]);
    for path in spec.depends.get("oci").into_iter().flatten() {
        let text = path.to_str().ok_or("OCI input path must be UTF-8")?;
        let literal = !text.is_empty() && !text.contains(['*', '?', '\\']);
        let normalized = path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
            && path
                .components()
                .map(|part| part.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/")
                == text;
        if !literal || !normalized {
            return Err(format!(
                "OCI input must be one normalized repository path: {text}"
            ));
        }
        paths.insert(text.to_string());
    }
    Ok(paths.into_iter().collect())
}

pub(in crate::command::ship) fn archive(spec: &Spec) -> Result<File, String> {
    let roots = inputs(spec)?;
    let snapshot = Snapshot::staged(&spec.root).map_err(|error| error.to_string())?;
    for root in &roots {
        if snapshot.seat(root).is_empty() {
            return Err(format!("OCI input has no tracked files: {root}"));
        }
    }
    let file =
        tempfile::tempfile().map_err(|error| format!("cannot open image context: {error}"))?;
    let mut archive = tar::Builder::new(file);
    for entry in snapshot
        .entries()
        .iter()
        .filter(|entry| selected(entry.path(), &roots))
    {
        append(&mut archive, entry)?;
    }
    let mut file = archive
        .into_inner()
        .map_err(|error| format!("cannot finish image context: {error}"))?;
    file.seek(SeekFrom::Start(0))
        .map_err(|error| format!("cannot rewind image context: {error}"))?;
    Ok(file)
}

fn selected(path: &str, roots: &[String]) -> bool {
    roots
        .iter()
        .any(|root| path == root || path.starts_with(&format!("{root}/")))
}

fn append(archive: &mut tar::Builder<File>, entry: &Entry) -> Result<(), String> {
    let mut header = tar::Header::new_gnu();
    header.set_uid(0);
    header.set_gid(0);
    header.set_mtime(0);
    match entry.mode() {
        "100644" | "100755" => {
            header.set_mode(if entry.mode() == "100755" {
                0o755
            } else {
                0o644
            });
            header.set_size(entry.bytes().len() as u64);
            header.set_cksum();
            archive.append_data(&mut header, entry.path(), entry.bytes())
        }
        "120000" => {
            let link = std::str::from_utf8(entry.bytes())
                .map_err(|error| format!("invalid OCI symlink: {error}"))?;
            header.set_entry_type(tar::EntryType::Symlink);
            header.set_mode(0o777);
            header.set_size(0);
            archive.append_link(&mut header, entry.path(), Path::new(link))
        }
        mode => {
            return Err(format!(
                "unsupported OCI input mode {mode}: {}",
                entry.path()
            ));
        }
    }
    .map_err(|error| format!("cannot archive OCI input {}: {error}", entry.path()))
}
