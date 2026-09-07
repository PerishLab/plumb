use super::environment::Environment;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub struct Execution {
    pub environment: Environment,
    root: PathBuf,
    tools: BTreeMap<String, Tool>,
}

#[derive(Serialize)]
struct Tool {
    path: PathBuf,
    digest: String,
}

impl Execution {
    pub fn new(environment: Environment, programs: &[String], root: &Path) -> Result<Self, String> {
        let root = root
            .canonicalize()
            .map_err(|error| format!("cannot resolve execution root: {error}"))?;
        let mut tools = BTreeMap::new();
        for program in programs {
            let path = which::which_in(program, environment.get("PATH"), &root)
                .map_err(|error| format!("cannot resolve tool {program}: {error}"))?;
            let digest = fingerprint(&path)?;
            tools.insert(program.clone(), Tool { path, digest });
        }
        Ok(Self {
            environment,
            root,
            tools,
        })
    }

    pub fn evidence(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(&(self.environment.evidence(), &self.tools))
            .map_err(|error| format!("cannot encode execution evidence: {error}"))
    }

    pub fn output(&self, argv: &[String]) -> Result<Output, String> {
        let (program, args) = argv
            .split_first()
            .ok_or_else(|| "execution requires a program".to_string())?;
        let mut command = self.command(program)?;
        command.args(args);
        super::process::capture(&mut command)
    }

    pub fn command(&self, program: &str) -> Result<Command, String> {
        for (name, tool) in &self.tools {
            let path = which::which_in(name, self.environment.get("PATH"), &self.root)
                .map_err(|error| format!("cannot resolve tool {name}: {error}"))?;
            if path != tool.path || fingerprint(&path)? != tool.digest {
                return Err(format!("resolved tool {name} changed before execution"));
            }
        }
        let tool = self
            .tools
            .get(program)
            .ok_or_else(|| format!("execution has no resolved tool {program}"))?;
        let mut command = crate::config::detached(&tool.path);
        self.environment.apply(&mut command);
        command.current_dir(&self.root);
        Ok(command)
    }
}

fn fingerprint(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path)
        .map_err(|error| format!("cannot read resolved tool {}: {error}", path.display()))?;
    let mut digest = Sha256::new();
    let mut bytes = [0; 8192];
    loop {
        let count = file
            .read(&mut bytes)
            .map_err(|error| format!("cannot read resolved tool {}: {error}", path.display()))?;
        if count == 0 {
            break;
        }
        digest.update(&bytes[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}
