use std::path::{Path, PathBuf};
use std::process::Output;

pub(crate) struct Index {
    pub root: PathBuf,
    source: PathBuf,
    _claim: plumb::integration::staging::Claim,
}

impl Index {
    pub(crate) fn new(source: &Path, tree: &str) -> Result<Self, String> {
        let commit = record(source, tree)?;
        let base = plumb::integration::staging::root()
            .ok_or_else(|| "cannot stage guard worktree: no PLUMB_HOME".to_string())?;
        std::fs::create_dir_all(&base)
            .map_err(|error| format!("cannot create {}: {error}", base.display()))?;
        crate::execution::signal::watch();
        let claim = plumb::integration::staging::Claim::retain(&base, source)?;
        plumb::integration::staging::reap(&base);
        let root = claim.root.clone();
        if root.exists() {
            clean(&root)?;
            plumb::integration::staging::lock(source, &root);
            git(
                &root,
                &[
                    "checkout",
                    "--detach",
                    "--quiet",
                    "--no-overwrite-ignore",
                    &commit,
                ],
                "refresh Guard source",
            )?;
        } else {
            plumb::integration::staging::unlock(source, &root);
            let _ = git(source, &["worktree", "prune"], "prune absent worktrees");
            let output = plumb::config::detached("git")
                .arg("-C")
                .arg(source)
                .args([
                    "worktree", "add", "--detach", "--quiet", "--lock", "--reason",
                ])
                .arg(plumb::integration::staging::REASON)
                .arg(&root)
                .arg(&commit)
                .output()
                .map_err(|error| format!("cannot create guard worktree: {error}"))?;
            text(output, "create guard worktree")?;
        }
        Ok(Self {
            root,
            source: source.to_path_buf(),
            _claim: claim,
        })
    }

    pub(super) fn settle(&self) -> Result<(), String> {
        if !git(
            &self.root,
            &["diff", "--name-only"],
            "inspect resolved Guard source",
        )?
        .is_empty()
        {
            return Err("package resolution left unstaged Guard inputs".into());
        }
        let tree = git(&self.root, &["write-tree"], "capture resolved Guard source")?;
        if tree
            != git(
                &self.root,
                &["rev-parse", "HEAD^{tree}"],
                "read Guard source tree",
            )?
        {
            let commit = record(&self.root, &tree)?;
            let head = git(&self.root, &["rev-parse", "HEAD"], "read Guard source HEAD")?;
            git(
                &self.root,
                &["update-ref", "HEAD", &commit, &head],
                "bind resolved Guard source",
            )?;
        }
        Ok(())
    }
}

impl Drop for Index {
    fn drop(&mut self) {
        if self._claim.verify() && clean(&self.root).is_err() {
            plumb::integration::staging::unlock(&self.source, &self.root);
            let _ = plumb::config::detached("git")
                .arg("-C")
                .arg(&self.source)
                .args(["worktree", "remove", "--force"])
                .arg(&self.root)
                .output();
        }
    }
}

fn clean(root: &Path) -> Result<(), String> {
    if !git(
        root,
        &["status", "--porcelain", "--untracked-files=no"],
        "inspect retained Guard source",
    )?
    .is_empty()
    {
        return Err("retained Guard source has unverified tracked changes".into());
    }
    Ok(())
}

fn record(source: &Path, tree: &str) -> Result<String, String> {
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
    text(output, "record staged tree")
}

pub(super) struct Execution<'a> {
    pub execution: Option<&'a plumb::config::Execution>,
    pub observation: Option<crate::observation::Frame>,
}

#[locus::trace(with = execution.observation.as_ref().map(|(engine, context)| (*engine, context)))]
pub(super) fn execute(
    root: &Path,
    argv: &[String],
    execution: &Execution<'_>,
) -> Result<(), String> {
    crate::diagnostic::measure(operation(argv), || execution.run(root, argv))
}

fn operation(argv: &[String]) -> &'static str {
    match (
        argv.first().map(String::as_str),
        argv.get(1).map(String::as_str),
    ) {
        (Some("cargo"), Some("fmt")) => "cargo.fmt",
        (Some("cargo"), Some("clippy")) => "cargo.clippy",
        (Some("cargo"), Some("check")) => "cargo.check",
        (Some("cargo"), Some("test")) => "cargo.test",
        (Some("cargo"), Some("run")) => "cargo.run",
        (Some("pnpm"), _) => "pnpm.command",
        (Some("plumb"), _) => "plumb.command",
        (Some("ectropy"), _) => "ectropy.command",
        _ => "guard.command",
    }
}

impl Execution<'_> {
    fn run(&self, root: &Path, argv: &[String]) -> Result<(), String> {
        let Execution {
            execution: context, ..
        } = *self;
        let (program, args) = argv
            .split_first()
            .ok_or_else(|| "guard action has no command".to_string())?;
        let managed =
            context.is_some_and(|context| context.environment.get("CARGO_TARGET_DIR").is_some());
        let cache = (program == "cargo" && !managed)
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
                if managed {
                    "cargo"
                } else {
                    crate::execution::family(program)
                },
                root,
                &context.environment,
            )?;
        }
        let compiler = if program == "cargo" || managed {
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
        let mut child = command
            .spawn()
            .map_err(|error| format!("cannot run {}: {error}", argv.join(" ")))?;
        let status = crate::execution::signal::wait(&mut child, argv)?;
        crate::diagnostic::event(
            "command.exit",
            || serde_json::json!({"code":status.code(),"success":status.success()}),
        );
        drop(compiler);
        if status.success() {
            if let Some(cache) = &cache {
                cache.settle();
            }
            Ok(())
        } else {
            Err(format!("{} failed with {status}", argv.join(" ")))
        }
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

pub(super) fn text(output: Output, action: &str) -> Result<String, String> {
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(format!(
            "cannot {action}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

pub(super) fn unchanged(index: &Index) -> Result<(), String> {
    if !git(
        &index.root,
        &["diff", "HEAD", "--name-only"],
        "verify Guard inputs remained unchanged",
    )?
    .is_empty()
    {
        return Err("Guard command changed a tracked verification input".into());
    }
    Ok(())
}
