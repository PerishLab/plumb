use std::path::Path;
use std::process::Command;

pub(super) fn authority(
    held: &plumb::rig::Authority,
    product: &str,
) -> Result<plumb::rig::Authority, String> {
    let bucket = format!("perish-{product}-releases");
    if !held.bucket.is_empty() && held.bucket != bucket {
        return Err(format!(
            "release publish authority must target derived bucket {bucket}"
        ));
    }
    let actual = crate::command::release::storage::fingerprint(&held.endpoint);
    if actual != held.fingerprint {
        return Err(format!(
            "release publish authority fingerprint drift: expected {}, got {actual}",
            held.fingerprint
        ));
    }
    let mut derived = held.clone();
    derived.bucket = bucket;
    Ok(derived)
}

pub(super) fn projection() -> String {
    "Cargo.toml#/workspace/package/version".into()
}

pub(super) fn sources(spec: &crate::shape::release::Spec) -> Result<Vec<String>, String> {
    let mut roots = super::sources::read(&spec.root)?;
    if let Some(depends) = spec.depends.get("binary") {
        roots.extend(depends.iter().map(|path| display(path)));
    }
    Ok(roots.into_iter().collect())
}

fn display(path: &Path) -> String {
    path.components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

pub(super) fn pnpm(root: &std::path::Path, install: bool) -> Result<(), String> {
    if !install || !root.join("pnpm-lock.yaml").is_file() {
        return Ok(());
    }
    let store = root.join("target/pnpm-store");
    command(
        root,
        "pnpm",
        &[
            "install",
            "--frozen-lockfile",
            "--store-dir",
            &store.to_string_lossy(),
        ],
    )
}

fn command(root: &std::path::Path, program: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .current_dir(root)
        .status()
        .map_err(|error| format!("cannot run {program}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "{program} failed while materializing a ship request"
        ))
    }
}
