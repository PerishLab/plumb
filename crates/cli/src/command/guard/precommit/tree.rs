use std::path::{Path, PathBuf};
use std::process::Output;

pub(crate) struct Index {
    pub root: PathBuf,
    source: PathBuf,
}

impl Index {
    pub(super) fn new(source: &Path, tree: &str) -> Result<Self, String> {
        let parent = plumb::config::detached("git")
            .arg("-C")
            .arg(source)
            .args(["rev-parse", "--verify", "HEAD"])
            .output()
            .map_err(|error| format!("cannot run git to read HEAD: {error}"))?;
        let mut record = plumb::config::detached("git");
        record.arg("-C").arg(source).args(["commit-tree", tree]);
        if parent.status.success() {
            record
                .arg("-p")
                .arg(String::from_utf8_lossy(&parent.stdout).trim());
        }
        let mut message = "plumb staged guard".to_string();
        if parent.status.success() {
            let declared = plumb::rig::Rig::resolve(None)
                .map_err(|error| error.to_string())?
                .release
                .version;
            if let Some(datum) = plumb::datum::Tree(source).capture(&declared)? {
                message.push_str(&format!("\n\n{}", datum.trailer()?));
            }
        }
        let output = record
            .args(["-m", &message])
            .output()
            .map_err(|error| format!("cannot run git to record staged tree: {error}"))?;
        let commit = text(output, "record staged tree")?;
        let base = plumb::config::value("PLUMB_HOME")
            .map(PathBuf::from)
            .or_else(|| plumb::config::data("plumb"))
            .ok_or_else(|| "cannot stage guard worktree: no PLUMB_HOME".to_string())?
            .join("tmp");
        std::fs::create_dir_all(&base)
            .map_err(|error| format!("cannot create {}: {error}", base.display()))?;
        let temporary = tempfile::Builder::new()
            .prefix("guard-")
            .tempdir_in(&base)
            .map_err(|error| format!("cannot reserve guard worktree: {error}"))?;
        let root = temporary.path().to_path_buf();
        temporary
            .close()
            .map_err(|error| format!("cannot prepare guard worktree: {error}"))?;
        let output = plumb::config::detached("git")
            .arg("-C")
            .arg(source)
            .args(["worktree", "add", "--detach", "--quiet"])
            .arg(&root)
            .arg(&commit)
            .output()
            .map_err(|error| format!("cannot create guard worktree: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "cannot create guard worktree: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        Ok(Self {
            root,
            source: source.to_path_buf(),
        })
    }
}

impl Drop for Index {
    fn drop(&mut self) {
        let _ = plumb::config::detached("git")
            .arg("-C")
            .arg(&self.source)
            .args(["worktree", "remove", "--force"])
            .arg(&self.root)
            .output();
    }
}

pub(super) struct Execution<'a> {
    pub execution: Option<&'a plumb::config::Execution>,
}

pub(super) fn execute(
    root: &Path,
    argv: &[String],
    execution: &Execution<'_>,
) -> Result<(), String> {
    let Execution { execution: context } = *execution;
    let (program, args) = argv
        .split_first()
        .ok_or_else(|| "guard action has no command".to_string())?;
    let cache = (program == "cargo")
        .then(|| super::cargo::Lease::new(root))
        .transpose()?;
    let mut command = match context {
        Some(context) => context.command(program)?,
        None => plumb::config::detached(program),
    };
    if let Some(cache) = &cache {
        crate::cargo::configure(&mut command);
        command.env("CARGO_TARGET_DIR", &cache.root);
    }
    if let Some(context) = context {
        crate::execution::inspect(
            crate::execution::family(program),
            root,
            &context.environment,
        )?;
    }
    let compiler = if program == "cargo" {
        crate::cargo::cache(context)?
    } else {
        None
    };
    if let Some(compiler) = &compiler {
        compiler.apply(&mut command);
    }
    command
        .args(args)
        .current_dir(root)
        .env_remove("GIT_INDEX_FILE");
    let status = command
        .status()
        .map_err(|error| format!("cannot run {}: {error}", argv.join(" ")))?;
    if status.success() {
        if let Some(compiler) = compiler {
            compiler.finish()?;
        }
        Ok(())
    } else {
        Err(format!("{} failed with {status}", argv.join(" ")))
    }
}

pub(crate) fn git(root: &Path, args: &[&str], action: &str) -> Result<String, String> {
    let output = plumb::config::detached("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|error| format!("cannot run git to {action}: {error}"))?;
    text(output, action)
}

fn text(output: Output, action: &str) -> Result<String, String> {
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(format!(
            "cannot {action}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}
