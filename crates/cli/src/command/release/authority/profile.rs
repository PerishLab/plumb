use clap::Args;
use plumb::{
    cli::Root,
    forgejo::{Remote, git},
};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    fs,
    io::Write as _,
    path::{Path, PathBuf},
};
#[derive(Args)]
pub struct Input {
    #[command(flatten)]
    target: Root,
    #[arg(long, help = "Mode-0600 seat for the token's one-time secret")]
    escrow: Option<PathBuf>,
    #[arg(long, help = "Apply the ordered plan until every resource is ready")]
    pub(super) apply: bool,
    #[arg(long)]
    pub(super) json: bool,
}
#[derive(Clone)]
pub(super) struct Model {
    pub product: String,
    pub token: String,
    pub escrow: PathBuf,
    pub remote: Remote,
}
impl Model {
    pub fn read(input: Input) -> Result<Self, String> {
        let root = PathBuf::from(&input.target.root)
            .canonicalize()
            .map_err(|error| format!("cannot resolve {}: {error}", input.target.root))?;
        let product = super::super::product(&root)?;
        let escrow = input
            .escrow
            .unwrap_or_else(|| root.join(".local/release-registry.env"));
        let escrow = if escrow.is_absolute() {
            escrow
        } else {
            root.join(escrow)
        };
        let remote = git::remote(&root, "")?;
        Ok(Self {
            token: format!("{product}-release-registry-v1"),
            product,
            escrow,
            remote,
        })
    }

    pub fn repository(&self) -> String {
        format!("{}/{}", self.remote.owner, self.remote.repo)
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum Action {
    Capability,
    Repository,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum Status {
    Ready,
    Change,
    Deferred,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Observation {
    pub tokens: Vec<plumb::forgejo::Token>,
    pub escrow: Option<View>,
    pub exact: bool,
    pub secrets: std::collections::BTreeSet<String>,
    pub binding: Option<u64>,
}

const ID: &str = "REGISTRY_TOKEN_ID";
const NAME: &str = "REGISTRY_TOKEN_NAME";
const LAST: &str = "REGISTRY_TOKEN_LAST_EIGHT";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct View {
    pub id: u64,
    pub name: String,
    pub last: String,
}

pub struct Escrow {
    pub id: u64,
    pub name: String,
    pub last: String,
    pub secret: String,
}

pub struct Seat<'a>(&'a Path);

impl<'a> Seat<'a> {
    pub fn new(path: &'a Path) -> Self {
        Self(path)
    }

    pub fn load(&self) -> Result<Option<Escrow>, String> {
        let Some(fields) = File(self.0).read()? else {
            return Ok(None);
        };
        let names = [ID, NAME, LAST, super::SECRET];
        if fields.len() != names.len() || !fields.keys().all(|name| names.contains(&name.as_str()))
        {
            return Err(format!("invalid registry escrow {}", self.0.display()));
        }
        let id = fields[ID]
            .parse::<u64>()
            .ok()
            .filter(|id| *id > 0)
            .ok_or_else(|| format!("invalid registry escrow {}", self.0.display()))?;
        let held = Escrow {
            id,
            name: fields[NAME].clone(),
            last: fields[LAST].clone(),
            secret: fields[super::SECRET].clone(),
        };
        if held.name.is_empty() || held.last.len() != 8 {
            return Err(format!("invalid registry escrow {}", self.0.display()));
        }
        if held.secret.len() <= 8 || held.secret.chars().any(char::is_whitespace) {
            return Err(format!("invalid registry escrow {}", self.0.display()));
        }
        Ok(Some(held))
    }

    pub fn write(&self, held: &Escrow) -> Result<(), String> {
        File(self.0).persist(
            &format!(
                "{ID}={}\n{NAME}={}\n{LAST}={}\n{}={}",
                held.id,
                held.name,
                held.last,
                super::SECRET,
                held.secret
            ),
            false,
        )
    }

    pub fn binding(&self) -> Binding {
        Binding(self.0.with_extension("bound"))
    }

    pub fn retire(&self) -> Result<(), String> {
        remove(self.0)?;
        self.binding().retire()
    }
}

impl Escrow {
    pub fn view(&self) -> View {
        View {
            id: self.id,
            name: self.name.clone(),
            last: self.last.clone(),
        }
    }

    pub fn identifies(&self, token: &plumb::forgejo::Token) -> bool {
        self.id == token.id && self.name == token.name && self.last == token.last
    }

    pub fn credential(&self) -> String {
        format!("Bearer {}", self.secret)
    }
}

pub struct Binding(PathBuf);

impl Binding {
    pub fn load(&self) -> Result<Option<u64>, String> {
        let Some(fields) = File(&self.0).read()? else {
            return Ok(None);
        };
        if fields.len() != 1 {
            return Err(format!("invalid registry binding {}", self.0.display()));
        }
        fields
            .get(ID)
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|id| *id > 0)
            .map(Some)
            .ok_or_else(|| format!("invalid registry binding {}", self.0.display()))
    }

    pub fn write(&self, id: u64) -> Result<(), String> {
        File(&self.0).persist(&format!("{ID}={id}"), true)
    }

    fn retire(&self) -> Result<(), String> {
        remove(&self.0)
    }
}

struct File<'a>(&'a Path);

impl File<'_> {
    fn read(&self) -> Result<Option<BTreeMap<String, String>>, String> {
        let text = match fs::read_to_string(self.0) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(format!("cannot read {}: {error}", self.0.display())),
        };
        mode(self.0)?;
        let mut fields = BTreeMap::new();
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            let (name, value) = line
                .split_once('=')
                .ok_or_else(|| format!("invalid registry state {}", self.0.display()))?;
            if value.is_empty() || fields.insert(name.to_string(), value.to_string()).is_some() {
                return Err(format!("invalid registry state {}", self.0.display()));
            }
        }
        Ok(Some(fields))
    }

    fn persist(&self, body: &str, replace: bool) -> Result<(), String> {
        let parent = self
            .0
            .parent()
            .ok_or_else(|| "registry state has no parent".to_string())?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
        let mut draft = tempfile::NamedTempFile::new_in(parent)
            .map_err(|error| format!("cannot create registry state draft: {error}"))?;
        writeln!(draft, "{body}")
            .map_err(|error| format!("cannot write registry state draft: {error}"))?;
        draft
            .as_file()
            .sync_all()
            .map_err(|error| format!("cannot sync registry state draft: {error}"))?;
        protect(draft.as_file())?;
        if replace {
            draft
                .persist(self.0)
                .map_err(|error| format!("cannot publish {}: {error}", self.0.display()))?;
        } else {
            draft
                .persist_noclobber(self.0)
                .map_err(|error| format!("cannot publish {}: {error}", self.0.display()))?;
        }
        Ok(())
    }
}

fn remove(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("cannot retire {}: {error}", path.display())),
    }
}

#[cfg(unix)]
fn mode(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt as _;
    let held = fs::metadata(path)
        .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?
        .permissions()
        .mode()
        & 0o777;
    if held == 0o600 {
        Ok(())
    } else {
        Err(format!("{} must be mode 600", path.display()))
    }
}

#[cfg(not(unix))]
fn mode(path: &Path) -> Result<(), String> {
    fs::metadata(path)
        .map(|_| ())
        .map_err(|error| format!("cannot inspect {}: {error}", path.display()))
}

#[cfg(unix)]
fn protect(file: &fs::File) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt as _;
    file.set_permissions(fs::Permissions::from_mode(0o600))
        .map_err(|error| format!("cannot protect registry state draft: {error}"))
}

#[cfg(not(unix))]
fn protect(_: &fs::File) -> Result<(), String> {
    Ok(())
}
