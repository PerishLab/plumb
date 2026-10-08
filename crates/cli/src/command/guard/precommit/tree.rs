use plumb::guard::Descriptor;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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
        record
            .env("GIT_AUTHOR_NAME", "Plumb Guard")
            .env("GIT_AUTHOR_EMAIL", "guard@plumb.invalid")
            .env("GIT_COMMITTER_NAME", "Plumb Guard")
            .env("GIT_COMMITTER_EMAIL", "guard@plumb.invalid");
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
        let base = plumb::integration::staging::root()
            .ok_or_else(|| "cannot stage guard worktree: no PLUMB_HOME".to_string())?;
        std::fs::create_dir_all(&base)
            .map_err(|error| format!("cannot create {}: {error}", base.display()))?;
        let temporary = tempfile::Builder::new()
            .prefix(plumb::integration::staging::PREFIX)
            .tempdir_in(&base)
            .map_err(|error| format!("cannot reserve guard worktree: {error}"))?;
        let root = temporary.path().to_path_buf();
        temporary
            .close()
            .map_err(|error| format!("cannot prepare guard worktree: {error}"))?;
        let _ = plumb::config::detached("git")
            .arg("-C")
            .arg(source)
            .args(["worktree", "prune"])
            .output();
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
    command.args(args).current_dir(root);
    command.stdout(std::io::stderr());
    unhook(&mut command);
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

fn unhook(command: &mut std::process::Command) {
    let held = command
        .get_envs()
        .filter_map(|(key, value)| value.map(|_| key.to_os_string()))
        .collect::<Vec<_>>();
    for (key, _) in std::env::vars_os() {
        if key.to_str().is_some_and(|key| key.starts_with("GIT_")) && !held.contains(&key) {
            command.env_remove(key);
        }
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

pub(super) fn bind(
    root: &Path,
    before: &super::branch::Snapshot,
    proof: &Descriptor,
) -> Result<String, String> {
    let mut message = tempfile::NamedTempFile::new()
        .map_err(|error| format!("cannot reserve Guard message: {error}"))?;
    message
        .write_all(
            git(
                root,
                &["show", "-s", "--format=%B", &before.head],
                "read HEAD message",
            )?
            .as_bytes(),
        )
        .map_err(|error| format!("cannot stage Guard message: {error}"))?;
    plumb::guard::attach(root, message.path())?;
    let parents = git(
        root,
        &["rev-list", "--parents", "-n", "1", &before.head],
        "read HEAD parent set",
    )?;
    let mut parts = parents.split_whitespace();
    if parts.next().unwrap_or_default() != before.head {
        return Err("cannot read the exact HEAD parent set".into());
    }
    let mut arguments = vec!["commit-tree".to_string(), proof.tree.clone()];
    for parent in parts {
        arguments.extend(["-p".to_string(), parent.to_string()]);
    }
    arguments.extend([
        "-F".to_string(),
        message.path().to_string_lossy().into_owned(),
    ]);
    let mut command = plumb::config::detached("git");
    command.arg("-C").arg(root).args(arguments);
    for (key, shape) in [
        ("GIT_AUTHOR_NAME", "%an"),
        ("GIT_AUTHOR_EMAIL", "%ae"),
        ("GIT_AUTHOR_DATE", "%aI"),
        ("GIT_COMMITTER_NAME", "%cn"),
        ("GIT_COMMITTER_EMAIL", "%ce"),
        ("GIT_COMMITTER_DATE", "%cI"),
    ] {
        command.env(
            key,
            git(
                root,
                &["show", "-s", &format!("--format={shape}"), &before.head],
                "read HEAD identity",
            )?,
        );
    }
    let head = output(command, "create refreshed Guard commit")?;
    git(
        root,
        &["update-ref", &before.branch, &head, &before.head],
        "bind refreshed Guard commit",
    )?;
    Ok(head)
}

fn output(mut command: Command, action: &str) -> Result<String, String> {
    let output = command
        .output()
        .map_err(|error| format!("cannot run git to {action}: {error}"))?;
    text(output, action)
}
