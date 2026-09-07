use super::super::value;
use std::path::Path;
use std::process::{Command, Output};

pub fn make(
    root: &Path,
    version: &str,
    published: &str,
    projection: &str,
) -> Result<String, String> {
    let base = read(
        "resolve main for rejoin",
        command(root, &["rev-parse", "origin/main^{commit}"])?,
    )?;
    let tree = read(
        "resolve main tree for rejoin",
        command(root, &["rev-parse", "origin/main^{tree}"])?,
    )?;
    let proof = plumb::guard::commit(root, &base)?;
    let message = format!(
        "Rejoin {version}\n\n{} {}",
        plumb::guard::TRAILER,
        proof.encode()?
    );
    let head = read(
        "make topology-only rejoin",
        command(
            root,
            &[
                "commit-tree",
                &tree,
                "-p",
                &base,
                "-p",
                published,
                "-m",
                &message,
            ],
        )?,
    )?;
    value::commit(&head)?;
    success(
        "verify topology-only rejoin tree",
        command(root, &["diff-tree", "--quiet", &base, &head])?,
    )?;
    proved(root, &head)?;
    let refspec = format!("{head}:refs/heads/{projection}");
    success(
        "push topology-only rejoin",
        command(root, &["push", "--force-with-lease", "origin", &refspec])?,
    )?;
    Ok(head)
}

pub fn proved(root: &Path, head: &str) -> Result<(), String> {
    let base = read(
        "resolve rejoin first parent",
        command(root, &["rev-parse", &format!("{head}^1")])?,
    )?;
    let inherited = plumb::guard::commit(root, &base)?;
    let carried = plumb::guard::commit(root, head)?;
    if inherited != carried {
        return Err("rejoin does not carry its main parent's exact Guard proof".into());
    }
    Ok(())
}

fn command(root: &Path, args: &[&str]) -> Result<Output, String> {
    Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| format!("cannot run git: {error}"))
}

fn success(action: &str, output: Output) -> Result<(), String> {
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "{action} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn read(action: &str, output: Output) -> Result<String, String> {
    success(action, output.clone())?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
