use std::collections::BTreeSet;
use std::path::Path;

pub mod changelog;
mod lock;
mod node;
mod operator;
mod pack;
pub mod pair;
mod policy;

pub use lock::{Found, Lock, locked, seal};

pub struct Shape {
    pub wrappers: BTreeSet<String>,
    pub hooks: BTreeSet<String>,
    pub dirs: BTreeSet<String>,
    pub actions: BTreeSet<String>,
    pub operator_tests: Vec<String>,
    pub block: Option<i64>,
    pub path: Option<i64>,
    pub grants: BTreeSet<String>,
    pub laws: bool,
    pub unread: Option<String>,
    pub lanes: BTreeSet<String>,
    pub ships: BTreeSet<String>,
    pub sites: BTreeSet<String>,
    pub ignore: String,
    pub listed: BTreeSet<String>,
    pub bounds: Vec<String>,
    pub root: std::path::PathBuf,
    pub locks: Vec<Lock>,
    pub version: Option<String>,
    pub inits: bool,
    pub rust: bool,
    pub runseal: bool,
    pub lane: Option<String>,
    pub edition: Option<String>,
    pub binary: bool,
    pub clap: bool,
    pub substrate: bool,
    pub deno: String,
    pub mint: Option<String>,
    pub packages: Vec<(String, String)>,
    pub node: Vec<(String, String)>,
    pub repo: Option<String>,
    pub named: Vec<(String, String)>,
    pub derived: Vec<String>,
    pub entries: Vec<String>,
    pub dispatch: Option<Vec<(&'static str, String)>>,
    pub web: Option<Vec<(&'static str, String)>>,
    pub policy: Vec<String>,
    pub guard: String,
    pub init: String,
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

    fn dirs(&self) -> BTreeSet<String> {
        let mut found = BTreeSet::new();
        let Ok(entries) = std::fs::read_dir(self.0) else {
            return found;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let skip = name.starts_with('.') || name == "target" || name == "node_modules";
            if entry.path().is_dir() && !skip {
                found.insert(name);
            }
        }
        found
    }

    fn ships(&self) -> BTreeSet<String> {
        let mut found = BTreeSet::new();
        if self.0.join("manage.sh").exists() {
            found.insert("binary".to_string());
        }
        if self.0.join("deno.json").exists() {
            found.insert("jsr".to_string());
        }
        for seat in ["packages"] {
            let Ok(entries) = std::fs::read_dir(self.0.join(seat)) else {
                continue;
            };
            for entry in entries.flatten() {
                if entry.path().join("deno.json").exists() {
                    found.insert("jsr".to_string());
                }
                if pack::minted(&entry.path().join("package.json")) {
                    found.insert("npm".to_string());
                }
            }
        }
        found
    }

    fn listed(&self) -> BTreeSet<String> {
        let mut found = BTreeSet::new();
        for seat in [".runseal/wrappers/init.ts", ".runseal/lib/init/init.ts"] {
            let Ok(text) = std::fs::read_to_string(self.0.join(seat)) else {
                continue;
            };
            for line in text.lines() {
                let Some(at) = line.find(".runseal/wrappers/") else {
                    continue;
                };
                let rest = &line[at + 18..];
                if let Some(end) = rest.find(".ts") {
                    found.insert(rest[..end].to_string());
                }
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

    fn denos(&self) -> String {
        std::fs::read_to_string(self.0.join(".runseal/deno.json")).unwrap_or_default()
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

pub fn read(root: &Path) -> Shape {
    let seat = Root(root);
    let laws = root.join("ectropy.toml");
    let text = std::fs::read_to_string(&laws).unwrap_or_default();
    let read = text.parse::<toml::Table>();
    let unread = read.as_ref().err().map(|error| error.to_string());
    let doc = read.ok().map(toml::Value::Table);
    let policy = if laws.exists() {
        doc.as_ref()
            .map(|doc| policy::check(root, doc))
            .unwrap_or_default()
    } else {
        Vec::new()
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
    Shape {
        wrappers: seat.names(".runseal/wrappers", ".ts"),
        hooks: seat.names(".runseal/hooks", ""),
        dirs: seat.dirs(),
        actions: operator::actions(root),
        operator_tests: operator::tests(root),
        block: limit("block"),
        path: limit("path"),
        grants,
        laws: laws.exists(),
        unread,
        lanes: seat.names(".forgejo/workflows", ".yml"),
        ships: seat.ships(),
        sites: pair::sites(root),
        ignore: std::fs::read_to_string(root.join(".gitignore")).unwrap_or_default(),
        rust: root.join("Cargo.toml").exists(),
        runseal: root.join(".runseal").is_dir(),
        lane: std::fs::read_to_string(root.join(".forgejo/workflows/guard.yml"))
            .ok()
            .map(|text| text.replace("\r\n", "\n")),
        edition: seat.edition(),
        binary: seat.binary(),
        clap: seat.manifests().iter().any(|text| {
            text.lines()
                .any(|line| line.trim_start().starts_with("clap"))
        }),
        substrate: seat.substrate(),
        deno: seat.denos(),
        mint: seat.mint(),
        packages: seat.packages(),
        node: node::read(root),
        repo: crate::anchor::Anchor(root).repo(),
        named: crate::anchor::Anchor(root).names(),
        derived: crate::anchor::Anchor(root).derives(),
        entries: crate::anchor::Anchor(root).entries(),
        dispatch: crate::dispatch::read(root),
        web: crate::web::read(root),
        policy,
        listed: seat.listed(),
        bounds: policy::bounds(doc.as_ref()),
        root: root.to_path_buf(),
        locks: lock::read(root),
        version: lock::held(root),
        inits: root.join(".runseal/wrappers/init.ts").exists()
            || root.join(".runseal/lib/init/init.ts").exists(),
        guard: std::fs::read_to_string(root.join(".runseal/wrappers/guard.ts")).unwrap_or_default(),
        init: std::fs::read_to_string(root.join(".runseal/wrappers/init.ts")).unwrap_or_default(),
    }
}

pub fn reconcile(root: &Path, text: &str) -> Result<String, String> {
    policy::render(root, text)
}
