use clap::Subcommand;
use plumb::rig::Rig;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Subcommand)]
pub enum Deed {
    #[command(about = "Install the exact configuration generation carried by a Plumb version")]
    Install {
        #[arg(default_value = ".")]
        root: String,
        #[arg(long)]
        version: Option<String>,
        #[arg(long)]
        path: Option<PathBuf>,
    },
}

pub fn run(deed: Deed) -> i32 {
    let held = match deed {
        Deed::Install {
            root,
            version,
            path,
        } => install(Path::new(&root), version.as_deref(), path.as_deref()),
    };
    match held {
        Ok(message) => {
            println!("{message}");
            0
        }
        Err(error) => {
            eprintln!("plumb configuration: {error}");
            1
        }
    }
}

fn install(root: &Path, version: Option<&str>, over: Option<&Path>) -> Result<String, String> {
    let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let source = rig.rules.source.trim_end_matches('/');
    let version = version.unwrap_or(plumb::version!("PLUMB"));
    if version != plumb::version!("PLUMB") && over.is_none() {
        return Err(format!(
            "non-running configuration {version} requires an explicit --path"
        ));
    }
    let channel = crate::command::release::channel(version)?;
    let generation = plumb::depot::v3::Generation::latest(plumb::depot::v3::Query {
        source,
        product: "plumb",
        channel: &channel,
        version,
        kind: plumb::depot::v3::Kind::Configuration,
    })?
    .ok_or_else(|| format!("depot carries no plumb configuration {version}"))?;
    let mut bodies = BTreeMap::new();
    for object in &generation.manifest.objects {
        bodies.insert(object.path.clone(), generation.read(&object.path)?);
    }
    let pointer = generation.pointer;
    let bundle = plumb::depot::v3::Bundle {
        manifest: generation.manifest,
        bodies,
    };
    let seat = match over {
        Some(path) => path.to_path_buf(),
        None => plumb::depot::root(Path::new(""))?,
    };
    let generation = plumb::depot::v3::install(&seat, &pointer, &bundle)?;
    let hooks = if over.is_none() {
        crate::command::precommit::project(root)?
    } else {
        None
    };
    Ok(format!(
        "installed plumb configuration {version} {} into {}{}",
        pointer.generation,
        generation.display(),
        hooks.map(|held| format!("\n{held}")).unwrap_or_default()
    ))
}
