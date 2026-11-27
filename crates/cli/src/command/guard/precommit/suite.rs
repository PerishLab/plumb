use super::super::workflow::tree::Tree;
use plumb::packages::Channels;

pub(super) struct Preparation {
    pub name: String,
    pub input: String,
    pub commands: Vec<Vec<String>>,
    pub probes: Vec<plumb::rule::Probe>,
    pub tools: Vec<String>,
    pub manifest: Option<String>,
    pub paths: Vec<String>,
}

pub(super) struct Catalog<'a> {
    pub tree: &'a Tree,
    pub product: &'a str,
}

impl Catalog<'_> {
    pub(super) fn checks(&self) -> Result<Vec<Preparation>, String> {
        let tree = self.tree;
        let manifest = tree.text("plumb.toml")?;
        plumb::windtun::parse(manifest.as_deref().unwrap_or_default())?;
        let mut held = manifest
            .as_deref()
            .map(crate::shape::workflow::parse)
            .unwrap_or_default();
        if let Some(error) = &held.refusal {
            return Err(error.clone());
        }
        let paths: Vec<_> = tree.paths().collect();
        if held.keys.is_empty() {
            let ectropy = crate::shape::workflow::governed(manifest.as_deref(), &paths);
            held.keys = crate::shape::workflow::inferred(
                tree.has("Cargo.toml"),
                tree.has("pnpm-lock.yaml"),
                tree.has("plumb.toml"),
                ectropy,
            )
            .keys;
        }
        held.validate()?;
        for key in &mut held.keys {
            crate::shape::workflow::include(key, tree.has("plumb.toml"), tree.has("ectropy.toml"));
        }
        let mut checks = Vec::new();
        for key in held.keys.iter().filter(|key| key.lane() == "guard") {
            let name = key.name();
            let commands = self.commands(tree, self.product, &name)?;
            if commands.is_empty() {
                continue;
            }
            let input = tree.digest(key);
            let matched = tree.selection(&key.paths);
            checks.push(Preparation {
                probes: held.probes(&name)?,
                tools: held.tools(&name)?,
                name,
                input,
                commands,
                manifest: manifest.clone(),
                paths: matched.into_iter().map(|(path, _)| path.clone()).collect(),
            });
        }
        if checks.is_empty() {
            return Err("staged tree implies no guard action".into());
        }
        Ok(checks)
    }

    fn commands(&self, tree: &Tree, product: &str, name: &str) -> Result<Vec<Vec<String>>, String> {
        let line = |text: &str| text.split_whitespace().map(str::to_string).collect();
        Ok(match name {
            "guard/rust" => vec![
                line("cargo fmt --all --check"),
                line("cargo clippy --locked --all-targets -- -D warnings"),
                line("cargo check --locked --workspace --all-targets --release"),
            ],
            "guard/test" => vec![line("cargo test --locked")],
            "guard/web" => web(tree)?,
            "guard/plumb" if product == "plumb" => {
                vec![line("cargo run --quiet --locked --bin plumb -- doctor .")]
            }
            "guard/plumb" => vec![line("plumb doctor .")],
            "guard/ectropy" if product == "ectropy" => {
                vec![line("cargo run --quiet --locked --bin ectropy -- .")]
            }
            "guard/ectropy" => vec![line("ectropy .")],
            _ => Vec::new(),
        })
    }
}

fn web(tree: &Tree) -> Result<Vec<Vec<String>>, String> {
    let mut held = vec![vec![
        "pnpm".into(),
        "install".into(),
        "--offline".into(),
        "--frozen-lockfile".into(),
    ]];
    if tree.has("biome.json") {
        held.push(vec!["pnpm".into(), "biome".into(), "ci".into(), ".".into()]);
    }
    held.push(vec![
        "pnpm".into(),
        "-r".into(),
        "--if-present".into(),
        "run".into(),
        "typecheck".into(),
    ]);
    held.push(vec!["pnpm".into(), "-r".into(), "test".into()]);
    let mut sites = Vec::new();
    for path in tree.paths().filter(|path| manifest(path)) {
        let text = tree
            .text(path)?
            .ok_or_else(|| format!("cannot read staged manifest {path}"))?;
        let doc: serde_json::Value = serde_json::from_str(&text)
            .map_err(|_| format!("invalid staged package manifest {path}"))?;
        if doc.pointer("/scripts/build").is_some() {
            let name = doc
                .get("name")
                .and_then(serde_json::Value::as_str)
                .filter(|name| !name.is_empty())
                .ok_or_else(|| format!("staged build manifest {path} requires a package name"))?;
            sites.push(name.to_string());
        }
    }
    sites.sort();
    for site in sites {
        held.push(vec!["pnpm".into(), "--filter".into(), site, "build".into()]);
    }
    Ok(held)
}

fn manifest(path: &str) -> bool {
    path.strip_prefix("apps/")
        .and_then(|path| path.strip_suffix("/package.json"))
        .is_some_and(|name| !name.is_empty() && !name.contains('/'))
}

pub(super) fn prepare(
    root: &std::path::Path,
    commands: &mut [Vec<String>],
) -> Result<(Vec<String>, plumb::packages::Policy), String> {
    let policy = plumb::packages::Policy::read(root)?;
    let mut flags = policy.flags.clone();
    let home = plumb::config::value("PLUMB_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| plumb::config::data("plumb"))
        .ok_or("cannot own the Guard pnpm cache: no PLUMB_HOME")?;
    flags.push(plumb::packages::cache(&home)?);
    flags.push("--config.verify-deps-before-run=error".into());
    for command in commands
        .iter_mut()
        .filter(|command| command.first().is_some_and(|program| program == "pnpm"))
    {
        command.splice(1..1, flags.iter().cloned());
    }
    Ok((flags, policy))
}
pub(super) fn bind(
    tree: &Tree,
    (action, product): (&str, &str),
    input: String,
    channels: Option<&Channels>,
) -> Result<String, String> {
    let Some(channels) = channels else {
        return Ok(input);
    };
    let files: &[&str] = match action {
        "guard/rust" | "guard/test" => &["Cargo.toml", "Cargo.lock"],
        "guard/web" => &["package.json", "pnpm-lock.yaml"],
        "guard/plumb" => &["Cargo.toml", "Cargo.lock", "package.json", "pnpm-lock.yaml"],
        "guard/ectropy" if product == "ectropy" => &["Cargo.toml", "Cargo.lock"],
        _ => return Ok(input),
    };
    let roots = tree
        .paths()
        .filter(|path| {
            path.rsplit('/')
                .next()
                .is_some_and(|name| files.contains(&name))
        })
        .map(str::to_string)
        .collect::<Vec<_>>();
    let observed = if action == "guard/plumb" {
        channels.packages()
    } else {
        &[]
    };
    let bytes = serde_json::to_vec(&(input, tree.selection(&roots), observed))
        .map_err(|error| error.to_string())?;
    Ok(plumb::depot::sha(&bytes))
}

pub(super) fn operation(name: &str) -> &'static str {
    match name {
        "guard/rust" => "guard.rust",
        "guard/lock" => "guard.lock",
        "guard/test" => "guard.test",
        "guard/web" => "guard.web",
        "guard/plumb" => "guard.plumb",
        "guard/ectropy" => "guard.ectropy",
        _ => "guard.action",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned() {
        let tree = Tree::default();
        let catalog = Catalog {
            tree: &tree,
            product: "product",
        };
        assert_eq!(
            catalog.commands(&tree, "product", "guard/test").unwrap(),
            vec![vec!["cargo".to_string(), "test".into(), "--locked".into()]],
            "Wharf skips its release cargo suite because guard/test runs exactly this on a virtual workspace (PerishLab/wharf#127); change both together"
        );
    }
}
