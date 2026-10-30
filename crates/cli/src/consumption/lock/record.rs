use serde::Serialize;

pub const SCHEMA: &str = "plumb.lock.inspection/v1";

pub const LIMITS: [&str; 7] = [
    "only Rust sources are read; every other code file is listed as unscanned",
    "roles come from resolved paths and inferred receiver types, never from text matches",
    "receivers that cannot be typed are reported as unknown when they call a lock-named method",
    "read and write calls are classified only on resolved lock types",
    "macro bodies other than parsed expression macros are reported as unknown when they carry lock tokens",
    "indirect consumption follows first-party crates in the workspace and the Perish registry only",
    "attribute macros other than the Radicula guard are read as written",
];

#[derive(Serialize)]
pub struct Inspection {
    pub schema: &'static str,
    pub tool: Tool,
    pub repository: Repository,
    pub dependencies: Vec<Dependency>,
    pub findings: Vec<Finding>,
    pub unknown: Vec<Gap>,
    pub unscanned: Vec<Skip>,
    pub limits: Vec<&'static str>,
    pub summary: Summary,
}

#[derive(Serialize)]
pub struct Tool {
    pub name: &'static str,
    pub version: String,
    pub parser: &'static str,
}

#[derive(Serialize)]
pub struct Repository {
    pub name: String,
    pub commit: Option<String>,
    pub dirty: bool,
}

#[derive(Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    pub source: String,
    pub scanned: bool,
}

#[derive(Clone, Serialize)]
pub struct Finding {
    pub path: String,
    pub line: usize,
    pub column: usize,
    pub item: String,
    pub form: &'static str,
    pub role: &'static str,
    pub lifecycle: Option<&'static str>,
    pub call: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub carrier: Option<String>,
    pub hold: Option<Hold>,
    pub domain: Domain,
    pub via: Vec<String>,
    pub cfg: Vec<String>,
    pub context: &'static str,
}

#[derive(Clone, Serialize, Default)]
pub struct Hold {
    pub form: &'static str,
    pub name: Option<String>,
    pub awaited: bool,
    pub across: bool,
    pub release: Option<usize>,
}

#[derive(Clone, Serialize, Default)]
pub struct Domain {
    pub receiver: Option<String>,
    pub owner: Option<String>,
    pub origin: Option<String>,
    pub key: Option<String>,
}

#[derive(Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Gap {
    pub path: String,
    pub line: usize,
    pub column: usize,
    pub item: String,
    pub reason: String,
    pub text: String,
}

#[derive(Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Skip {
    pub path: String,
    pub reason: String,
}

#[derive(Serialize)]
pub struct Summary {
    pub files: usize,
    pub rust: usize,
    pub findings: usize,
    pub roles: std::collections::BTreeMap<&'static str, usize>,
    pub unknown: usize,
    pub unscanned: usize,
    pub complete: bool,
}
