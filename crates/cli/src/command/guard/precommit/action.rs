use plumb::guard::{Action, Descriptor};
use sha2::{Digest as _, Sha256};
use std::path::{Path, PathBuf};

use super::super::workflow::tree::Tree;
use super::tree::{self, Index};

struct Check {
    proof: Action,
    commands: Vec<Vec<String>>,
}

struct Catalog<'a> {
    root: &'a Path,
}

pub(super) fn prove(root: &Path) -> Result<Descriptor, String> {
    let tree = tree::git(root, &["write-tree"], "read staged tree")?;
    if let Ok(proof) = plumb::guard::staged(root, &tree) {
        return Ok(proof);
    }
    let checks = Catalog { root }.checks()?;
    let pending = checks
        .iter()
        .filter(|check| !cached(&check.proof))
        .collect::<Vec<_>>();
    if !pending.is_empty() {
        let index = Index::new(root, &tree)?;
        for check in pending {
            eprintln!("guard {}", check.proof.name);
            for command in &check.commands {
                tree::execute(&index.root, command)?;
            }
            cache(&check.proof)?;
        }
    }
    let proof = Descriptor::new(
        root,
        tree,
        checks.into_iter().map(|check| check.proof).collect(),
    )?;
    plumb::guard::stage(root, &proof)?;
    Ok(proof)
}

impl Catalog<'_> {
    fn checks(&self) -> Result<Vec<Check>, String> {
        let tree = Tree::read(self.root, None)?;
        let mut held = crate::shape::workflow::read(self.root);
        if let Some(error) = held.refusal {
            return Err(error);
        }
        if held.keys.is_empty() {
            held = crate::shape::workflow::inferred(
                tree.has("Cargo.toml"),
                tree.has("pnpm-lock.yaml"),
                tree.has("plumb.toml"),
                tree.has("ectropy.toml"),
            );
        }
        let product = self.product().unwrap_or_default();
        let mut checks = Vec::new();
        for key in held.keys.iter().filter(|key| key.lane() == "guard") {
            let name = key.name();
            let commands = self.commands(&product, &name)?;
            if commands.is_empty() {
                continue;
            }
            let input = tree.digest(key);
            let world = world(&name, &input, &commands)?;
            checks.push(Check {
                proof: Action { name, input, world },
                commands,
            });
        }
        if checks.is_empty() {
            return Err("staged tree implies no guard action".into());
        }
        Ok(checks)
    }

    fn commands(&self, product: &str, name: &str) -> Result<Vec<Vec<String>>, String> {
        let cargo = |args: &[&str]| {
            std::iter::once("cargo".to_string())
                .chain(args.iter().map(|arg| arg.to_string()))
                .collect()
        };
        Ok(match name {
            "guard/rust" => vec![
                cargo(&["fmt", "--all", "--check"]),
                cargo(&["clippy", "--all-targets", "--", "-D", "warnings"]),
                cargo(&[
                    "check",
                    "--locked",
                    "--workspace",
                    "--all-targets",
                    "--release",
                ]),
            ],
            "guard/test" => vec![cargo(&["test", "--locked"])],
            "guard/web" => self.web()?,
            "guard/plumb" if product == "plumb" => vec![cargo(&[
                "run", "--quiet", "--locked", "--bin", "plumb", "--", "doctor", ".",
            ])],
            "guard/plumb" => vec![vec!["plumb".into(), "doctor".into(), ".".into()]],
            "guard/ectropy" if product == "ectropy" => vec![cargo(&[
                "run", "--quiet", "--locked", "--bin", "ectropy", "--", ".",
            ])],
            "guard/ectropy" => vec![vec!["ectropy".into(), ".".into()]],
            _ => Vec::new(),
        })
    }

    fn web(&self) -> Result<Vec<Vec<String>>, String> {
        let mut held = vec![
            vec!["corepack".into(), "enable".into()],
            vec!["pnpm".into(), "install".into(), "--frozen-lockfile".into()],
        ];
        if self.root.join("biome.json").is_file() {
            held.push(vec!["pnpm".into(), "biome".into(), "ci".into(), ".".into()]);
        }
        held.push(vec![
            "pnpm".into(),
            "-r".into(),
            "exec".into(),
            "tsc".into(),
            "--noEmit".into(),
        ]);
        held.push(vec!["pnpm".into(), "-r".into(), "test".into()]);
        let Ok(entries) = std::fs::read_dir(self.root.join("apps")) else {
            return Ok(held);
        };
        let mut sites = Vec::new();
        for entry in entries.flatten() {
            let Ok(text) = std::fs::read_to_string(entry.path().join("package.json")) else {
                continue;
            };
            let Ok(doc) = serde_json::from_str::<serde_json::Value>(&text) else {
                continue;
            };
            if doc.pointer("/scripts/build").is_some()
                && let Some(name) = doc.get("name").and_then(serde_json::Value::as_str)
            {
                sites.push(name.to_string());
            }
        }
        sites.sort();
        for site in sites {
            held.push(vec!["pnpm".into(), "--filter".into(), site, "build".into()]);
        }
        Ok(held)
    }

    fn product(&self) -> Result<String, String> {
        let text = std::fs::read_to_string(self.root.join("plumb.toml"))
            .map_err(|error| format!("cannot read plumb.toml: {error}"))?;
        let doc: toml::Table = text
            .parse()
            .map_err(|error| format!("cannot parse plumb.toml: {error}"))?;
        Ok(doc
            .get("release")
            .and_then(|release| release.get("product"))
            .and_then(toml::Value::as_str)
            .unwrap_or_default()
            .to_string())
    }
}

fn world(name: &str, input: &str, commands: &[Vec<String>]) -> Result<String, String> {
    let mut sponge = Sha256::new();
    sponge.update(name.as_bytes());
    sponge.update([0]);
    sponge.update(input.as_bytes());
    sponge.update([0]);
    sponge.update(serde_json::to_vec(commands).map_err(|error| error.to_string())?);
    sponge.update([0]);
    sponge.update(plumb::version!("PLUMB").as_bytes());
    if let Some(commit) = plumb::commit!("PLUMB") {
        sponge.update([0]);
        sponge.update(commit.as_bytes());
    }
    sponge.update([0]);
    sponge.update(plumb::depot::rules()?.mark().as_bytes());
    sponge.update([0]);
    sponge.update(plumb::config::platform().as_bytes());
    for tool in tools(name) {
        sponge.update([0]);
        sponge.update(tool.as_bytes());
        sponge.update([0]);
        sponge.update(version(tool)?.as_bytes());
    }
    Ok(format!("{:x}", sponge.finalize()))
}

fn tools(name: &str) -> &'static [&'static str] {
    match name {
        "guard/rust" | "guard/test" => &["cargo", "rustc"],
        "guard/web" => &["node", "pnpm"],
        "guard/ectropy" => &["ectropy"],
        "guard/plumb" => &["plumb"],
        _ => &[],
    }
}

fn version(tool: &str) -> Result<String, String> {
    let output = plumb::config::detached(tool)
        .arg("--version")
        .output()
        .map_err(|error| format!("cannot run {tool} --version: {error}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(format!(
            "{tool} --version failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn seat(proof: &Action) -> Result<PathBuf, String> {
    let home = plumb::config::value("PLUMB_HOME")
        .map(PathBuf::from)
        .or_else(|| plumb::config::data("plumb"))
        .ok_or_else(|| "cannot cache guard action: no PLUMB_HOME".to_string())?;
    Ok(home
        .join("proof")
        .join("guard")
        .join("actions")
        .join(format!("{}.json", proof.world)))
}

fn cached(proof: &Action) -> bool {
    seat(proof)
        .ok()
        .and_then(|path| std::fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice::<Action>(&bytes).ok())
        .as_ref()
        == Some(proof)
}

fn cache(proof: &Action) -> Result<(), String> {
    let path = seat(proof)?;
    let parent = path
        .parent()
        .ok_or_else(|| "guard action has no cache parent".to_string())?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(proof).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("cannot write {}: {error}", path.display()))
}
