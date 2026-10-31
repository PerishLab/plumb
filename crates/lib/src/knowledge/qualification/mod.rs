mod archive;
mod current;
mod receipt;
mod registry;
mod remote;
mod source;
mod unique;
mod wire;
mod world;

pub use remote::Remote;
use serde::Serialize;
use serde_json::{Value, json};

pub type Result<T> = std::result::Result<T, Fault>;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Qualified,
    Refused,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Fault {
    pub state: State,
    pub reason: String,
}

impl Fault {
    pub fn refused(reason: impl Into<String>) -> Self {
        Self {
            state: State::Refused,
            reason: reason.into(),
        }
    }

    pub fn unknown(reason: impl Into<String>) -> Self {
        Self {
            state: State::Unknown,
            reason: reason.into(),
        }
    }
}

impl std::fmt::Display for Fault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.reason.fmt(f)
    }
}

impl std::error::Error for Fault {}

pub trait Read {
    fn bytes(&self, url: &str, limit: usize) -> Result<Vec<u8>>;
    fn image(&self, reference: &str) -> Result<String>;
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub schema: &'static str,
    pub subject: Value,
    pub state: State,
    pub qualified: bool,
    pub platform: &'static str,
    pub association: Option<bool>,
    pub promotion: bool,
    pub evidence: Option<Value>,
    pub reason: Option<String>,
}

pub struct Inspector<R> {
    reader: R,
}

impl<R: Read> Inspector<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    pub fn inspect(&self, repository: &str, marker: &str) -> Report {
        let result = self.verify(repository, marker);
        let (state, evidence, reason) = match result {
            Ok(value) => (State::Qualified, Some(value), None),
            Err(fault) => (fault.state, None, Some(fault.reason)),
        };
        Report {
            schema: "plumb.windtun.qualification/v1",
            subject: json!({"repository": repository, "marker": marker}),
            qualified: state == State::Qualified,
            state,
            platform: "linux-x86_64",
            association: None,
            promotion: false,
            evidence,
            reason,
        }
    }
}
