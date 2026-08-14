use super::super::release::generator;
use super::super::release::model::Spec;
use super::{Arm, Promotion, Recovery};
use plumb::forgejo::{Client, git};
use serde_json::{Value, json};
use std::path::Path;

const BASE: &str = "993698cbfbf826c791f26ef5da9cb194f712e3f8";
const FIX: &str = "4781b396694ac631b5c1bd7ac5833ceaa8244448";
const MAIN: &str = "5a3b56b46ed0e66aa0e0b74dfda16077011a0854";

pub fn run(deed: Recovery, spec: &Spec) -> Result<String, String> {
    generator::contract(spec)?;
    let root = git::root()?;
    let remote = git::remote(&root, "")?;
    if remote.owner != "PerishLab" || remote.repo != generator::PRODUCT {
        return Err(format!(
            "self-hosting recovery requires origin {}, got {}/{}",
            generator::REPOSITORY,
            remote.owner,
            remote.repo
        ));
    }
    match deed {
        Recovery::Arm(input) => arm(&root, input),
        Recovery::Beta { caller, dry } => beta(remote, caller, dry),
        Recovery::Stable(input) => stable(remote, input),
    }
}

fn arm(root: &Path, input: Arm) -> Result<String, String> {
    commits([
        ("--generator", &input.generator),
        ("--release", &input.release),
        ("--actions", &input.actions),
        ("--caller", &input.caller),
    ])?;
    distinct([
        BASE,
        FIX,
        MAIN,
        &input.generator,
        &input.release,
        &input.actions,
        &input.caller,
    ])?;
    Topology(root).prove(&input.generator, &input.release)?;
    let mode = if input.dry { "validated" } else { "armed" };
    Ok(format!(
        "{mode} exact recovery {} G={} R={} A={} C={}",
        generator::RELEASE_BRANCH,
        input.generator,
        input.release,
        input.actions,
        input.caller
    ))
}

fn beta(remote: plumb::forgejo::Remote, caller: String, dry: bool) -> Result<String, String> {
    commit("--caller", &caller)?;
    dispatch(DispatchInput {
        remote,
        caller,
        inputs: json!({"phase": "beta"}),
        version: generator::BETA_VERSION,
        dry,
    })
}

fn stable(remote: plumb::forgejo::Remote, input: Promotion) -> Result<String, String> {
    commit("--caller", &input.caller)?;
    digest(&input.digest)?;
    dispatch(DispatchInput {
        remote,
        caller: input.caller,
        inputs: json!({"phase": "stable", "beta_sha256": input.digest}),
        version: generator::STABLE_VERSION,
        dry: input.dry,
    })
}

struct DispatchInput {
    remote: plumb::forgejo::Remote,
    caller: String,
    inputs: Value,
    version: &'static str,
    dry: bool,
}

fn dispatch(input: DispatchInput) -> Result<String, String> {
    let DispatchInput {
        remote,
        caller,
        inputs,
        version,
        dry,
    } = input;
    if dry {
        return Ok(format!(
            "would dispatch {} at C={caller} for {version}",
            generator::WORKFLOW
        ));
    }
    let run = Client::new(remote)?.dispatch(generator::WORKFLOW, &caller, inputs)?;
    let id = run
        .get("id")
        .and_then(Value::as_u64)
        .ok_or_else(|| "Forgejo did not expose the dispatched recovery run ID".to_string())?;
    Ok(format!(
        "dispatched {version} recovery run {id} at C={caller}"
    ))
}

struct Topology<'a>(&'a Path);

impl Topology<'_> {
    fn prove(&self, generator: &str, release: &str) -> Result<(), String> {
        self.success(&["merge-base", "--is-ancestor", BASE, FIX], "S0..F")?;
        self.success(&["merge-base", "--is-ancestor", FIX, MAIN], "F..M")?;
        self.success(&["merge-base", "--is-ancestor", MAIN, generator], "M..G")?;
        let release = self.text(&["rev-parse", &format!("{release}^{{tree}}")])?;
        let generator = self.text(&["rev-parse", &format!("{generator}^{{tree}}")])?;
        if release != generator {
            return Err("release and generator trees disagree".into());
        }
        Ok(())
    }

    fn success(&self, args: &[&str], subject: &str) -> Result<(), String> {
        let output = self.command(args)?;
        if output.status.success() {
            Ok(())
        } else {
            Err(format!("recovery topology does not prove {subject}"))
        }
    }

    fn text(&self, args: &[&str]) -> Result<String, String> {
        let output = self.command(args)?;
        if !output.status.success() {
            return Err(format!(
                "cannot inspect recovery topology: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn command(&self, args: &[&str]) -> Result<std::process::Output, String> {
        std::process::Command::new("git")
            .args(args)
            .current_dir(self.0)
            .output()
            .map_err(|error| format!("cannot run git: {error}"))
    }
}

fn commits<const N: usize>(values: [(&str, &str); N]) -> Result<(), String> {
    for (name, value) in values {
        commit(name, value)?;
    }
    Ok(())
}

fn commit(name: &str, value: &str) -> Result<(), String> {
    super::value::commit(value)
        .map_err(|_| format!("{name} requires one full lowercase commit SHA"))
}

fn digest(value: &str) -> Result<(), String> {
    if value.len() == 64
        && value
            .bytes()
            .all(|held| held.is_ascii_hexdigit() && !held.is_ascii_uppercase())
    {
        Ok(())
    } else {
        Err("--beta-sha256 requires one lowercase SHA-256 digest".into())
    }
}

fn distinct<const N: usize>(values: [&str; N]) -> Result<(), String> {
    let unique = values
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    if unique.len() == N {
        Ok(())
    } else {
        Err("recovery commit identities must be distinct".into())
    }
}
