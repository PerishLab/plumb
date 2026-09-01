use clap::Subcommand;
use plumb::rig::Rig;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

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
    let route =
        plumb::depot::v3::Route::new(&channel, plumb::depot::v3::Kind::Configuration, version);
    let key = plumb::depot::v3::latest(route)?;
    let pointer = plumb::depot::v3::Pointer::parse(&pull(&format!("{source}/{key}"))?)?;
    let standing = (
        pointer.product.as_str(),
        pointer.channel.as_str(),
        pointer.version.as_str(),
        pointer.kind,
    );
    let wanted = (
        "plumb",
        channel.as_str(),
        version,
        plumb::depot::v3::Kind::Configuration,
    );
    if standing != wanted {
        return Err(format!(
            "exact depot pointer does not name plumb configuration {version}"
        ));
    }
    let expected = plumb::depot::v3::manifest(source, route, &pointer.generation)?;
    if pointer.manifest.url != expected {
        return Err("configuration pointer names another depot authority".into());
    }
    let body = pull(&pointer.manifest.url)?;
    let manifest = plumb::depot::v3::Manifest::parse(&body)?;
    pointer.bind(&manifest, &body)?;
    let base = pointer
        .manifest
        .url
        .strip_suffix(plumb::depot::v3::LEAF)
        .expect("a valid manifest URL ends in its leaf");
    let mut bodies = BTreeMap::new();
    for object in &manifest.objects {
        bodies.insert(
            object.path.clone(),
            pull(&format!("{base}objects/{}", object.path))?,
        );
    }
    let bundle = plumb::depot::v3::Bundle { manifest, bodies };
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

fn pull(url: &str) -> Result<Vec<u8>, String> {
    let body = tempfile::NamedTempFile::new()
        .map_err(|error| format!("cannot stage a configuration fetch: {error}"))?;
    let output = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--connect-timeout",
            "5",
            "--max-time",
            "15",
            "--retry",
            "1",
            "--retry-all-errors",
            "--output",
        ])
        .arg(body.path())
        .arg(url)
        .output()
        .map_err(|error| format!("cannot run curl: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cannot fetch {url}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    std::fs::read(body.path())
        .map_err(|error| format!("cannot read the configuration fetch: {error}"))
}
