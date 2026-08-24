pub(crate) mod dispatch;
pub mod web;
use std::collections::BTreeSet;
use std::path::Path;

pub mod changelog;
pub(crate) mod dependency;
mod forge;
pub mod lane;
pub mod layout;
mod node;
pub(crate) mod operator;
mod pack;
pub mod pair;
pub(crate) mod policy;
pub mod workflow;

pub use crate::judge::finding::Found;
pub use dependency::{Dependencies, Dependency};

pub struct Shape {
    pub wrappers: BTreeSet<String>,
    pub dirs: BTreeSet<String>,
    pub block: Option<i64>,
    pub path: Option<i64>,
    pub grants: BTreeSet<String>,
    pub laws: bool,
    pub unread: Option<String>,
    pub lanes: BTreeSet<String>,
    pub drift: Vec<String>,
    pub release: pair::Release,
    pub ships: BTreeSet<String>,
    pub sites: BTreeSet<String>,
    pub ignore: String,
    pub bounds: Vec<(String, bool)>,
    pub components: bool,
    pub rust: bool,
    pub runseal: bool,
    pub guards: Vec<(String, String)>,
    pub edition: Option<String>,
    pub binary: bool,
    pub clap: bool,
    pub substrate: bool,
    pub dependencies: Dependencies,
    pub mint: Option<String>,
    pub packages: Vec<(String, String)>,
    pub node: Vec<(String, String)>,
    pub repo: Option<String>,
    pub named: Vec<(String, String)>,
    pub derived: Vec<String>,
    pub entries: Vec<String>,
    pub dispatch: Option<dispatch::Evidence>,
    pub web: Option<web::Evidence>,
    pub(crate) policy: Option<policy::Evidence>,
    pub guard: String,
    pub layout: layout::Read,
}

struct Root<'a>(&'a Path);

impl Root<'_> {
    fn names(&self, under: &str, suffix: &str) -> BTreeSet<String> {
        let mut found = BTreeSet::new();
        let Ok(entries) = std::fs::read_dir(self.0.join(under)) else {
            return found;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(stem) = name.strip_suffix(suffix) {
                found.insert(stem.to_string());
            }
        }
        found
    }

    fn dirs(snapshot: &plumb::snapshot::Snapshot) -> BTreeSet<String> {
        let mut found = BTreeSet::new();
        for entry in snapshot.entries() {
            let name = entry
                .path()
                .split_once('/')
                .map(|(head, _)| head)
                .or_else(|| (entry.mode() == "160000").then_some(entry.path()));
            let Some(name) = name else {
                continue;
            };
            let skip = name.starts_with('.') || name == "target" || name == "node_modules";
            if !skip {
                found.insert(name.to_string());
            }
        }
        found
    }

    fn packages(&self) -> Vec<(String, String)> {
        let mut held = Vec::new();
        let Ok(entries) = std::fs::read_dir(self.0.join("packages")) else {
            return held;
        };
        for entry in entries.flatten() {
            let dir = entry.file_name().to_string_lossy().to_string();
            if let Some(name) = pack::name(&entry.path()) {
                held.push((dir, name));
            }
        }
        held
    }

    fn mint(&self) -> Option<String> {
        if let Ok(text) = std::fs::read_to_string(self.0.join("deno.json"))
            && text.contains("\"exports\"")
        {
            return pack::field(&text, "name");
        }
        if pack::minted(&self.0.join("package.json"))
            && let Ok(text) = std::fs::read_to_string(self.0.join("package.json"))
        {
            return pack::field(&text, "name");
        }
        None
    }

    fn manifests(&self) -> Vec<String> {
        let mut held = Vec::new();
        for seat in ["Cargo.toml", "app/Cargo.toml"] {
            if let Ok(text) = std::fs::read_to_string(self.0.join(seat)) {
                held.push(text);
            }
        }
        if let Ok(entries) = std::fs::read_dir(self.0.join("crates")) {
            for entry in entries.flatten() {
                if let Ok(text) = std::fs::read_to_string(entry.path().join("Cargo.toml")) {
                    held.push(text);
                }
            }
        }
        held
    }

    fn binary(&self) -> bool {
        if self.manifests().iter().any(|text| text.contains("[[bin]]")) {
            return true;
        }
        let mut seats = vec![self.0.to_path_buf(), self.0.join("app")];
        if let Ok(entries) = std::fs::read_dir(self.0.join("crates")) {
            for entry in entries.flatten() {
                seats.push(entry.path());
            }
        }
        seats.iter().any(|seat| {
            seat.join("Cargo.toml").is_file()
                && (seat.join("src/main.rs").is_file() || seat.join("src/bin").is_dir())
        })
    }

    fn substrate(&self) -> bool {
        std::fs::read_to_string(self.0.join("Cargo.lock"))
            .map(|lock| lock.contains("name = \"plumb\""))
            .unwrap_or(false)
    }

    fn edition(&self) -> Option<String> {
        let text = std::fs::read_to_string(self.0.join("Cargo.toml")).ok()?;
        let doc = text.parse::<toml::Table>().ok()?;
        let held = doc
            .get("workspace")
            .and_then(|value| value.get("package"))
            .or_else(|| doc.get("package"));
        held.and_then(|value| value.get("edition"))
            .and_then(toml::Value::as_str)
            .map(str::to_string)
    }
}

pub fn capture(
    root: &Path,
    snapshot: &Result<plumb::snapshot::Snapshot, plumb::snapshot::Refusal>,
) -> Shape {
    let seat = Root(root);
    let operator = operator::Operator(root);
    let guarded = operator.guard();
    let laws = root.join("ectropy.toml");
    let text = std::fs::read_to_string(&laws).unwrap_or_default();
    let read = text.parse::<toml::Table>();
    let unread = read.as_ref().err().map(|error| error.to_string());
    let doc = read.ok().map(toml::Value::Table);
    let policy = if laws.exists() {
        doc.as_ref()
            .map(|doc| policy::Evidence::read(root, doc.clone()))
    } else {
        None
    };
    let limit = |key: &str| {
        doc.as_ref()
            .and_then(|value| value.get("limit"))
            .and_then(|value| value.get(key))
            .and_then(toml::Value::as_integer)
    };
    let mut grants = BTreeSet::new();
    if let Some(list) = doc
        .as_ref()
        .and_then(|value| value.get("grant"))
        .and_then(toml::Value::as_array)
    {
        for entry in list {
            if let Some(name) = entry.get("syntax").and_then(toml::Value::as_str) {
                grants.insert(name.to_string());
            }
        }
    }
    let held = layout::read(root, snapshot.as_ref());
    let lanes = seat.names(".forgejo/workflows", ".yml");
    let paired = pair::Root(root);
    let release = paired.release(&lanes);
    let ships = release.attachments.clone();
    let bounds = policy::bounds(doc.as_ref())
        .into_iter()
        .map(|path| {
            let exists = root.join(&path).exists();
            (path, exists)
        })
        .collect();
    Shape {
        wrappers: seat.names(".runseal/wrappers", ".ts"),
        dirs: snapshot.as_ref().map(Root::dirs).unwrap_or_default(),
        block: limit("block"),
        path: limit("path"),
        grants,
        laws: laws.exists(),
        unread,
        lanes,
        drift: lane::Seat(root).drift(),
        release,
        ships,
        sites: paired.sites(),
        ignore: std::fs::read_to_string(root.join(".gitignore")).unwrap_or_default(),
        bounds,
        components: root.join("packages/components").is_dir(),
        rust: root.join("Cargo.toml").exists(),
        runseal: root.join("runseal.toml").is_file() || root.join(".runseal").is_dir(),
        guards: guarded.lanes,
        edition: seat.edition(),
        binary: seat.binary(),
        clap: seat.manifests().iter().any(|text| {
            text.lines()
                .any(|line| line.trim_start().starts_with("clap"))
        }),
        substrate: seat.substrate(),
        dependencies: dependency::read(root),
        mint: seat.mint(),
        packages: seat.packages(),
        node: node::read(root),
        repo: crate::anchor::Anchor(root).repo(),
        named: crate::anchor::Anchor(root).names(),
        derived: crate::anchor::Anchor(root).derives(),
        entries: crate::anchor::Anchor(root).entries(),
        dispatch: dispatch::read(root),
        web: web::read(root),
        policy,
        guard: guarded.source,
        layout: held,
    }
}

pub fn version(root: &Path) -> Option<String> {
    let cargo = std::fs::read_to_string(root.join("Cargo.toml")).unwrap_or_default();
    if let Some(found) = mark(&cargo) {
        return Some(found);
    }
    for name in ["deno.json", "package.json"] {
        let text = std::fs::read_to_string(root.join(name)).unwrap_or_default();
        if let Some(found) = quoted(&text) {
            return Some(found);
        }
    }
    None
}

fn mark(text: &str) -> Option<String> {
    text.lines()
        .find(|line| line.trim_start().starts_with("version = \""))
        .and_then(|line| line.split('"').nth(1))
        .map(str::to_string)
}

fn quoted(text: &str) -> Option<String> {
    let seat = text.find("\"version\"")?;
    text[seat..].split('"').nth(3).map(str::to_string)
}
